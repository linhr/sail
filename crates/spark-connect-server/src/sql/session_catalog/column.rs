use crate::schema::to_spark_data_type;
use crate::sql::session_catalog::SessionCatalogContext;
use arrow::datatypes::FieldRef;
use datafusion_common::{exec_datafusion_err, Result, TableReference};
use framework_common::unwrap_or;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TableColumnMetadata {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) data_type: String,
    pub(crate) nullable: bool,
    pub(crate) is_partition: bool,
    pub(crate) is_bucket: bool,
}

impl TableColumnMetadata {
    fn try_new(column: &FieldRef) -> Result<Self> {
        // TODO: avoid converting `SparkError` back to `DataFusionError`
        //   We should probably restructure the code.
        let data_type = to_spark_data_type(column.data_type())
            .and_then(|x| x.to_simple_string())
            .map_err(|e| exec_datafusion_err!("{}", e))?;
        Ok(Self {
            name: column.name().clone(),
            description: None, // TODO: support description
            data_type,
            nullable: column.is_nullable(),
            is_partition: false, // TODO: Add actual is_partition if available
            is_bucket: false,    // TODO: Add actual is_bucket if available
        })
    }
}

impl SessionCatalogContext<'_> {
    pub(crate) async fn list_table_columns(
        &self,
        table: TableReference,
    ) -> Result<Vec<TableColumnMetadata>> {
        let (catalog_name, database_name, table_name) = self.resolve_table_reference(table)?;
        let catalog_provider = unwrap_or!(
            self.ctx.catalog(catalog_name.as_ref()),
            return Ok(Vec::new())
        );
        let schema_provider = unwrap_or!(
            catalog_provider.schema(database_name.as_ref()),
            return Ok(Vec::new())
        );
        let table = unwrap_or!(
            schema_provider.table(table_name.as_ref()).await?,
            return Ok(Vec::new())
        );
        Ok(table
            .schema()
            .fields()
            .iter()
            .map(|column| TableColumnMetadata::try_new(column))
            .collect::<Result<Vec<_>>>()?)
    }
}
