use std::collections::HashMap;

use crate::{
    diagnostics::{ObjectType, QueryRootOperationType, UndefinedDefinition, UniqueDefinition},
    hir::RootOperationTypeDefinition,
    ApolloDiagnostic, ValidationDatabase,
};

pub fn check(db: &dyn ValidationDatabase) -> Vec<ApolloDiagnostic> {
    let mut diagnostics = Vec::new();

    // A GraphQL schema must have a Query root operation.
    if db.schema().query(db.upcast()).is_none() {
        if let Some(loc) = db.schema().loc() {
            let offset = loc.offset();
            let len = loc.node_len();
            diagnostics.push(ApolloDiagnostic::QueryRootOperationType(
                QueryRootOperationType {
                    src: db.source_code(loc.file_id()),
                    schema: (offset, len).into(),
                },
            ));
        }
    }

    let mut seen: HashMap<String, &RootOperationTypeDefinition> = HashMap::new();
    for op_type in db.schema().root_operation_type_definition().iter() {
        let name = op_type.named_type().name();

        // All root operations in a schema definition must be unique.
        //
        // Return a Unique Operation Definition error in case of a duplicate name.
        if let Some(prev_def) = seen.get(&name) {
            if prev_def.loc().is_some() && op_type.loc().is_some() {
                let prev_offset = prev_def.loc().unwrap().offset();
                let prev_node_len = prev_def.loc().unwrap().node_len();

                let offset = op_type.loc().unwrap().offset();
                let node_len = op_type.loc().unwrap().node_len();

                diagnostics.push(ApolloDiagnostic::UniqueDefinition(UniqueDefinition {
                    name: name.clone(),
                    ty: "root operation type definition".into(),
                    src: db.source_code(prev_def.loc().unwrap().file_id()),
                    original_definition: (prev_offset, prev_node_len).into(),
                    redefined_definition: (offset, node_len).into(),
                    help: Some(format!(
                        "`{name}` must only be defined once in this document."
                    )),
                }));
            }
        } else {
            seen.insert(name, op_type);
        }

        // Root Operation Named Type must be of Object Type.
        //
        // Return a Object Type error if it's any other type definition.
        let type_def = db.find_type_definition_by_name(op_type.named_type().name());
        if let Some(type_def) = type_def {
            if !type_def.is_object_type_definition() {
                let offset = op_type.loc().unwrap().offset();
                let node_len = op_type.loc().unwrap().node_len();

                diagnostics.push(ApolloDiagnostic::ObjectType(ObjectType {
                    name: op_type.named_type().name(),
                    ty: type_def.ty(),
                    src: db.source_code(op_type.loc().unwrap().file_id()),
                    definition: (offset, node_len).into(),
                    help: "root operation type must be of an Object Type.".into(),
                }))
            }
        } else if op_type.loc().is_some() {
            let offset = op_type.loc().unwrap().offset();
            let node_len = op_type.loc().unwrap().node_len();

            diagnostics.push(ApolloDiagnostic::UndefinedDefinition(UndefinedDefinition {
                ty: op_type.named_type().name(),
                src: db.source_code(op_type.loc().unwrap().file_id()),
                definition: (offset, node_len).into(),
            }))
        }
    }

    diagnostics
}
