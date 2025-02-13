use migration_core::migration_connector::{ConnectorError, ConnectorResult};
use once_cell::sync::OnceCell;
use quaint::{prelude::*, single::Quaint};
use url::Url;

pub(crate) async fn cockroach_setup(url: String, prisma_schema: &str) -> ConnectorResult<()> {
    let mut url = Url::parse(&url).map_err(ConnectorError::url_parse_error)?;
    let quaint_url = quaint::connector::PostgresUrl::new(url.clone()).unwrap();
    let db_name = quaint_url.dbname();
    let conn = create_admin_conn(&mut url).await?;

    let query = format!(
        r#"
        DROP DATABASE IF EXISTS "{db_name}";
        CREATE DATABASE "{db_name}";
        "#
    );

    conn.raw_cmd(&query).await.unwrap();

    crate::diff_and_apply(prisma_schema).await;

    drop_db_when_thread_exits(url, db_name);

    Ok(())
}

async fn create_admin_conn(url: &mut Url) -> ConnectorResult<Quaint> {
    url.set_path("/postgres");
    Ok(Quaint::new(url.as_ref()).await.unwrap())
}

fn drop_db_when_thread_exits(admin_url: Url, db_name: &str) {
    use std::{cell::RefCell, sync::mpsc, thread};
    use test_setup::runtime::run_with_thread_local_runtime as tok;

    // === Dramatis Personæ ===

    // DB_DROP_THREAD: A thread that drops databases.
    static DB_DROP_THREAD: OnceCell<mpsc::SyncSender<String>> = OnceCell::new();

    let sender = DB_DROP_THREAD.get_or_init(|| {
        let (sender, receiver) = mpsc::sync_channel::<String>(4096);

        thread::spawn(move || {
            let mut admin_url = admin_url;
            let conn = tok(create_admin_conn(&mut admin_url)).unwrap();

            // Receive new databases to drop.
            for msg in receiver.iter() {
                tok(conn.raw_cmd(&msg)).unwrap();
            }
        });

        sender
    });

    // NOTIFIER: a thread local that notifies DB_DROP_THREAD when dropped.
    struct Notifier(String, mpsc::SyncSender<String>);

    impl Drop for Notifier {
        fn drop(&mut self) {
            let message = std::mem::take(&mut self.0);

            self.1.send(message).unwrap();
        }
    }

    thread_local! {
        static NOTIFIER: RefCell<Option<Notifier>> = RefCell::new(None);
    }

    NOTIFIER.with(move |cell| {
        *cell.borrow_mut() = Some(Notifier(format!("DROP DATABASE \"{db_name}\""), sender.clone()));
    });
}
