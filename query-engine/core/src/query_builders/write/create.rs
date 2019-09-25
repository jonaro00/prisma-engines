use super::*;
use write_arguments::*;
use crate::{
    query_ast::*,
    query_graph::{Node, NodeRef, QueryGraph, QueryGraphDependency},
    ArgumentListLookup, ParsedField, ParsedInputMap, ReadOneRecordBuilder,
};
use connector::{
    filter::{RecordFinder},
};
use prisma_models::{ModelRef};
use std::{convert::TryInto, sync::Arc};

/// Creates a create record query and adds it to the query graph, together with it's nested queries and companion read query.
pub fn create_record(graph: &mut QueryGraph, model: ModelRef, mut field: ParsedField) -> QueryBuilderResult<()> {
    let id_field = model.fields().id();
    let data_argument = field.arguments.lookup("data").unwrap();
    let data_map: ParsedInputMap = data_argument.value.try_into()?;
    let create_node = create::create_record_node(graph, Arc::clone(&model), data_map)?;

    // Follow-up read query on the write
    let read_query = ReadOneRecordBuilder::new(field, model).build()?;
    let read_node = graph.create_node(Query::Read(read_query));

    graph.add_result_node(&read_node);
    graph.create_edge(
        &create_node,
        &read_node,
        QueryGraphDependency::ParentId(Box::new(|mut node, parent_id| {
            if let Node::Query(Query::Read(ReadQuery::RecordQuery(ref mut rq))) = node {
                let finder = RecordFinder {
                    field: id_field,
                    value: parent_id.unwrap(),
                };

                rq.record_finder = Some(finder);
            };

            node
        })),
    );

    Ok(())
}

pub fn create_record_node(
    graph: &mut QueryGraph,
    model: ModelRef,
    data_map: ParsedInputMap,
) -> QueryBuilderResult<NodeRef> {
    let create_args = WriteArguments::from(&model, data_map)?;
    let mut non_list_args = create_args.non_list;

    non_list_args.add_datetimes(Arc::clone(&model));

    let cr = CreateRecord {
        model,
        non_list_args,
        list_args: create_args.list,
    };

    let node = graph.create_node(Query::Write(WriteQuery::CreateRecord(cr)));

    for (relation_field, data_map) in create_args.nested {
        nested::connect_nested_query(graph, &node, relation_field, data_map)?;
    }

    Ok(node)
}