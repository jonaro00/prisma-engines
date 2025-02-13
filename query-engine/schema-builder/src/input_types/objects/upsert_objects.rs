use crate::constants::args;
use crate::input_types::fields::arguments::where_argument;
use crate::mutations::create_one;

use super::*;

pub(crate) fn nested_upsert_input_object(
    ctx: &mut BuilderContext<'_>,
    parent_field: &RelationFieldRef,
) -> Option<InputObjectTypeWeakRef> {
    if parent_field.is_list() {
        nested_upsert_list_input_object(ctx, parent_field)
    } else {
        nested_upsert_nonlist_input_object(ctx, parent_field)
    }
}

/// Builds "<x>UpsertWithWhereUniqueNestedInput" / "<x>UpsertWithWhereUniqueWithout<y>Input" input object types.
fn nested_upsert_list_input_object(
    ctx: &mut BuilderContext<'_>,
    parent_field: &RelationFieldRef,
) -> Option<InputObjectTypeWeakRef> {
    let related_model = parent_field.related_model();
    let where_object = filter_objects::where_unique_object_type(ctx, &related_model);
    let create_types = create_one::create_one_input_types(ctx, &related_model, Some(parent_field));
    let update_types = update_one_objects::update_one_input_types(ctx, &related_model, Some(parent_field));

    if where_object.into_arc().is_empty() || create_types.iter().all(|typ| typ.is_empty()) {
        return None;
    }

    let ident = Identifier::new_prisma(format!(
        "{}UpsertWithWhereUniqueWithout{}Input",
        related_model.name(),
        capitalize(parent_field.related_field().name())
    ));

    match ctx.get_input_type(&ident) {
        None => {
            let input_object = Arc::new(init_input_object_type(ident.clone()));
            ctx.cache_input_type(ident, input_object.clone());

            let fields = vec![
                input_field(ctx, args::WHERE, InputType::object(where_object), None),
                input_field(ctx, args::UPDATE, update_types, None),
                input_field(ctx, args::CREATE, create_types, None),
            ];

            input_object.set_fields(fields);
            Some(Arc::downgrade(&input_object))
        }
        x => x,
    }
}

/// Builds "<x>UpsertNestedInput" / "<x>UpsertWithout<y>Input" input object types.
fn nested_upsert_nonlist_input_object(
    ctx: &mut BuilderContext<'_>,
    parent_field: &RelationFieldRef,
) -> Option<InputObjectTypeWeakRef> {
    let related_model = parent_field.related_model();
    let create_types = create_one::create_one_input_types(ctx, &related_model, Some(parent_field));
    let update_types = update_one_objects::update_one_input_types(ctx, &related_model, Some(parent_field));

    if create_types.iter().all(|typ| typ.is_empty()) {
        return None;
    }

    let ident = Identifier::new_prisma(format!(
        "{}UpsertWithout{}Input",
        related_model.name(),
        capitalize(parent_field.related_field().name())
    ));

    match ctx.get_input_type(&ident) {
        None => {
            let input_object = Arc::new(init_input_object_type(ident.clone()));
            ctx.cache_input_type(ident, input_object.clone());

            let mut fields = vec![
                input_field(ctx, args::UPDATE, update_types, None),
                input_field(ctx, args::CREATE, create_types, None),
            ];

            if ctx.has_feature(PreviewFeature::ExtendedWhereUnique) {
                fields.push(where_argument(ctx, &related_model));
            }

            input_object.set_fields(fields);
            Some(Arc::downgrade(&input_object))
        }
        x => x,
    }
}
