use datafusion::datasource::TableProvider;
use std::fmt;
use std::sync::Arc;

use datafusion_common::{
    exec_err, Constraints, DFSchema, DFSchemaRef, Result, SchemaReference, TableReference,
};
use datafusion_expr::{CreateMemoryTable, DdlStatement, DropTable, LogicalPlan, TableType};
use framework_common::unwrap_or;

use crate::catalog::utils::match_pattern;
use crate::catalog::CatalogManager;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TableTypeName {
    #[allow(dead_code)]
    EXTERNAL,
    MANAGED,
    VIEW,
    TEMPORARY,
}

impl fmt::Display for TableTypeName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TableTypeName::EXTERNAL => write!(f, "EXTERNAL"),
            TableTypeName::MANAGED => write!(f, "MANAGED"),
            TableTypeName::VIEW => write!(f, "VIEW"),
            TableTypeName::TEMPORARY => write!(f, "TEMPORARY"),
        }
    }
}

impl From<TableType> for TableTypeName {
    fn from(table_type: TableType) -> Self {
        match table_type {
            TableType::Base => TableTypeName::MANAGED, // TODO: Could also be EXTERNAL
            TableType::Temporary => TableTypeName::TEMPORARY,
            TableType::View => TableTypeName::VIEW,
        }
    }
    // TODO: handle_execute_create_dataframe_view
    //  is currently creating a View table from DataFrame.
    //  Unsure if this would be considered a Temporary View Table or not.
    //  Spark's expectation is that a Temporary View Table is created.
}

impl From<TableTypeName> for TableType {
    fn from(catalog_table_type: TableTypeName) -> Self {
        match catalog_table_type {
            TableTypeName::EXTERNAL => TableType::Base,
            TableTypeName::MANAGED => TableType::Base,
            TableTypeName::VIEW => TableType::View,
            TableTypeName::TEMPORARY => TableType::Temporary,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TableMetadata {
    pub(crate) name: String,
    pub(crate) catalog: Option<String>,
    pub(crate) namespace: Option<Vec<String>>,
    pub(crate) description: Option<String>,
    pub(crate) table_type: String,
    pub(crate) is_temporary: bool,
}

impl TableMetadata {
    pub(crate) fn new(
        catalog_name: &str,
        database_name: &str,
        table_name: &str,
        table_provider: Arc<dyn TableProvider>,
    ) -> Self {
        let table_type: TableTypeName = table_provider.table_type().into();
        let (catalog, namespace) =
            if table_type == TableTypeName::TEMPORARY || table_type == TableTypeName::VIEW {
                // Temporary views in the Spark session do not have a catalog or namespace
                (None, None)
            } else {
                (
                    Some(catalog_name.to_string()),
                    Some(vec![database_name.to_string()]),
                )
            };
        Self {
            name: table_name.to_string(),
            catalog,
            namespace,
            description: None, // TODO: support description
            table_type: table_type.to_string(),
            is_temporary: table_type == TableTypeName::TEMPORARY,
        }
    }
}

impl<'a> CatalogManager<'a> {
    pub(crate) async fn create_memory_table(
        &self,
        table: TableReference,
        plan: Arc<LogicalPlan>,
    ) -> Result<()> {
        let ddl = LogicalPlan::Ddl(DdlStatement::CreateMemoryTable(CreateMemoryTable {
            name: table,
            constraints: Constraints::empty(), // TODO: Check if exists in options
            input: plan,
            if_not_exists: false,    // TODO: Check if exists in options
            or_replace: false,       // TODO: Check if exists in options
            column_defaults: vec![], // TODO: Check if exists in options
        }));
        // TODO: process the output
        _ = self.ctx.execute_logical_plan(ddl).await?;
        Ok(())
    }

    pub(crate) async fn get_table(&self, table: TableReference) -> Result<Option<TableMetadata>> {
        let (catalog_name, database_name, table_name) = self.resolve_table_reference(table)?;
        let catalog_provider = unwrap_or!(self.ctx.catalog(catalog_name.as_ref()), return Ok(None));
        let schema_provider = unwrap_or!(
            catalog_provider.schema(database_name.as_ref()),
            return Ok(None)
        );
        let table_provider = unwrap_or!(
            schema_provider.table(table_name.as_ref()).await?,
            return Ok(None)
        );
        Ok(Some(TableMetadata::new(
            catalog_name.as_ref(),
            database_name.as_ref(),
            table_name.as_ref(),
            table_provider,
        )))
    }

    pub(crate) async fn list_tables(
        &self,
        database: Option<SchemaReference>,
        table_pattern: Option<&str>,
    ) -> Result<Vec<TableMetadata>> {
        let (catalog_name, database_name) = self.resolve_database_reference(database)?;
        let catalog_provider = unwrap_or!(
            self.ctx.catalog(catalog_name.as_ref()),
            return Ok(Vec::new())
        );
        let schema_provider = unwrap_or!(
            catalog_provider.schema(database_name.as_ref()),
            return Ok(Vec::new())
        );
        let mut tables = Vec::new();
        for table_name in schema_provider.table_names() {
            if !match_pattern(table_name.as_str(), table_pattern) {
                continue;
            }
            let table_provider = unwrap_or!(schema_provider.table(&table_name).await?, continue);
            tables.push(TableMetadata::new(
                catalog_name.as_ref(),
                database_name.as_ref(),
                table_name.as_str(),
                table_provider,
            ));
        }
        Ok(tables)
        // TODO: Spark temporary views are session-scoped and are not associated with a catalog or database.
        //   We should create a "hidden" catalog provider and include the temporary views in the result.
        //   Spark *global* temporary views should be put in the `global_temp` database, and they will be
        //   included in the result if the database pattern matches `global_temp`.
        //   The `global_temp` database name can be configured via `spark.sql.globalTempDatabase`.
    }

    pub(crate) async fn drop_table(
        &self,
        table: TableReference,
        if_exists: bool,
        purge: bool,
    ) -> Result<()> {
        if purge {
            return exec_err!("DROP TABLE ... PURGE is not supported");
        }
        let ddl = LogicalPlan::Ddl(DdlStatement::DropTable(DropTable {
            name: table,
            if_exists,
            schema: DFSchemaRef::new(DFSchema::empty()),
        }));
        self.ctx.execute_logical_plan(ddl).await?;
        Ok(())
    }
}
