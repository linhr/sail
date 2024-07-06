use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::spec::data_type::Schema;
use crate::spec::expression::{
    CommonInlineUserDefinedFunction, CommonInlineUserDefinedTableFunction, Expr, ObjectName,
    SortOrder,
};
use crate::spec::literal::Literal;
use crate::spec::Identifier;

/// Unresolved logical plan node for the framework.
/// As a starting point, the definition matches the structure of the `Relation` message
/// in the Spark Connect protocol, but enum variants are added to include write commands
/// and plan analysis requests in Spark Connect. There are additional variants to
/// represent SQL constructs that are not part of the Spark Connect protocol.
///
/// For the plan nodes originating from the `Relation` message, there are a few minor differences.
///   1. We do not support raw SQL plan/expression strings as a plan node. Such SQL strings
///      should be parsed before creating the plan node.
///   2. We do not support schema strings or field metadata JSON strings. Such strings
///      should be parsed before creating the schema used in the plan node.
///   3. We use "schema" instead of "data type" in the plan node. The "schema" consists of
///      a list of named fields, each with a data type. In contrast, the Spark Connect
///      protocol does not have the concept of a schema, and instead uses a struct data type
///      whenever a schema is needed.
///   4. We do not use abbreviations (e.g. "exprs", "cols", "func", "db", "temp") in names.
///   5. We use Pascal Case for acronyms (e.g. "Na", "Df", "Udf") in enum variant names.
///   6. Some names are modified for naming consistency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Plan {
    Query(QueryPlan),
    Command(CommandPlan),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryPlan {
    #[serde(flatten)]
    pub node: QueryNode,
    pub plan_id: Option<i64>,
    pub source_info: Option<String>,
}

impl QueryPlan {
    pub fn new(node: QueryNode) -> Self {
        Self {
            node,
            plan_id: None,
            source_info: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandPlan {
    #[serde(flatten)]
    pub node: CommandNode,
    pub plan_id: Option<i64>,
    pub source_info: Option<String>,
}

impl CommandPlan {
    pub fn new(node: CommandNode) -> Self {
        Self {
            node,
            plan_id: None,
            source_info: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum QueryNode {
    Read {
        #[serde(flatten)]
        read_type: ReadType,
        is_streaming: bool,
    },
    Project {
        input: Option<Box<QueryPlan>>,
        expressions: Vec<Expr>,
    },
    Filter {
        input: Box<QueryPlan>,
        condition: Expr,
    },
    Join(Join),
    SetOperation(SetOperation),
    Sort {
        input: Box<QueryPlan>,
        order: Vec<SortOrder>,
        is_global: bool,
    },
    Limit {
        input: Box<QueryPlan>,
        skip: usize,
        limit: usize,
    },
    Aggregate(Aggregate),
    LocalRelation {
        data: Option<Vec<u8>>,
        schema: Option<Schema>,
    },
    Sample(Sample),
    Offset {
        input: Box<QueryPlan>,
        offset: usize,
    },
    Deduplicate(Deduplicate),
    Range(Range),
    SubqueryAlias {
        input: Box<QueryPlan>,
        alias: Identifier,
        qualifier: Vec<Identifier>,
    },
    Repartition {
        input: Box<QueryPlan>,
        num_partitions: usize,
        shuffle: bool,
    },
    ToDf {
        input: Box<QueryPlan>,
        column_names: Vec<Identifier>,
    },
    WithColumnsRenamed {
        input: Box<QueryPlan>,
        rename_columns_map: HashMap<Identifier, Identifier>,
    },
    Drop {
        input: Box<QueryPlan>,
        columns: Vec<Expr>,
        column_names: Vec<Identifier>,
    },
    Tail {
        input: Box<QueryPlan>,
        limit: usize,
    },
    WithColumns {
        input: Box<QueryPlan>,
        aliases: Vec<Expr>,
    },
    Hint {
        input: Box<QueryPlan>,
        name: String,
        parameters: Vec<Expr>,
    },
    Unpivot(Unpivot),
    ToSchema {
        input: Box<QueryPlan>,
        schema: Schema,
    },
    RepartitionByExpression {
        input: Box<QueryPlan>,
        partition_expressions: Vec<Expr>,
        num_partitions: Option<usize>,
    },
    MapPartitions {
        input: Box<QueryPlan>,
        function: CommonInlineUserDefinedFunction,
        is_barrier: bool,
    },
    CollectMetrics {
        input: Box<QueryPlan>,
        name: String,
        metrics: Vec<Expr>,
    },
    Parse(Parse),
    GroupMap(GroupMap),
    CoGroupMap(CoGroupMap),
    WithWatermark(WithWatermark),
    ApplyInPandasWithState(ApplyInPandasWithState),
    CachedLocalRelation {
        user_id: String,
        session_id: String,
        hash: String,
    },
    CachedRemoteRelation {
        relation_id: String,
    },
    CommonInlineUserDefinedTableFunction(CommonInlineUserDefinedTableFunction),
    // NA operations
    FillNa {
        input: Box<QueryPlan>,
        columns: Vec<Identifier>,
        values: Vec<Expr>,
    },
    DropNa {
        input: Box<QueryPlan>,
        columns: Vec<Identifier>,
        min_non_nulls: Option<usize>,
    },
    ReplaceNa {
        input: Box<QueryPlan>,
        columns: Vec<Identifier>,
        replacements: Vec<Replacement>,
    },
    // stat operations
    StatSummary {
        input: Box<QueryPlan>,
        statistics: Vec<String>,
    },
    StatDescribe {
        input: Box<QueryPlan>,
        columns: Vec<Identifier>,
    },
    StatCrosstab {
        input: Box<QueryPlan>,
        left_column: Identifier,
        right_column: Identifier,
    },
    StatCov {
        input: Box<QueryPlan>,
        left_column: Identifier,
        right_column: Identifier,
    },
    StatCorr {
        input: Box<QueryPlan>,
        left_column: Identifier,
        right_column: Identifier,
        method: String,
    },
    StatApproxQuantile {
        input: Box<QueryPlan>,
        columns: Vec<Identifier>,
        probabilities: Vec<f64>,
        relative_error: f64,
    },
    StatFreqItems {
        input: Box<QueryPlan>,
        columns: Vec<Identifier>,
        support: Option<f64>,
    },
    StatSampleBy {
        input: Box<QueryPlan>,
        column: Expr,
        fractions: Vec<Fraction>,
        seed: Option<i64>,
    },
    // extensions
    Empty {
        produce_one_row: bool,
    },
    WithParameters {
        input: Box<QueryPlan>,
        positional_arguments: Vec<Literal>,
        named_arguments: HashMap<String, Literal>,
    },
    Values(Vec<Vec<Expr>>),
    TableAlias {
        input: Box<QueryPlan>,
        name: Identifier,
        columns: Vec<Identifier>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum CommandNode {
    ShowString(ShowString),
    HtmlString(HtmlString),
    // TODO: add all the "analyze" requests
    // TODO: should streaming query request be added here?
    // catalog operations
    CurrentDatabase,
    SetCurrentDatabase {
        database_name: Identifier,
    },
    ListDatabases {
        catalog: Option<Identifier>,
        database_pattern: Option<String>,
    },
    ListTables {
        database: Option<ObjectName>,
        table_pattern: Option<String>,
    },
    ListFunctions {
        database: Option<ObjectName>,
        function_pattern: Option<String>,
    },
    ListColumns {
        table: ObjectName,
    },
    GetDatabase {
        database: ObjectName,
    },
    GetTable {
        table: ObjectName,
    },
    GetFunction {
        function: ObjectName,
    },
    DatabaseExists {
        database: ObjectName,
    },
    TableExists {
        table: ObjectName,
    },
    FunctionExists {
        function: ObjectName,
    },
    CreateTable {
        table: ObjectName,
        #[serde(flatten)]
        definition: TableDefinition,
    },
    DropTemporaryView {
        view: ObjectName,
        is_global: bool,
        if_exists: bool,
    },
    RecoverPartitions {
        table: ObjectName,
    },
    IsCached {
        table: ObjectName,
    },
    CacheTable {
        table: ObjectName,
        storage_level: Option<StorageLevel>,
    },
    UncacheTable {
        table: ObjectName,
    },
    ClearCache,
    RefreshTable {
        table: ObjectName,
    },
    RefreshByPath {
        path: String,
    },
    CurrentCatalog,
    SetCurrentCatalog {
        catalog_name: Identifier,
    },
    ListCatalogs {
        catalog_pattern: Option<String>,
    },
    CreateCatalog {
        catalog: Identifier,
        #[serde(flatten)]
        definition: CatalogDefinition,
    },
    // commands
    CreateDatabase {
        database: ObjectName,
        #[serde(flatten)]
        definition: DatabaseDefinition,
    },
    DropDatabase {
        database: ObjectName,
        if_exists: bool,
        cascade: bool,
    },
    RegisterFunction(CommonInlineUserDefinedFunction),
    RegisterTableFunction(CommonInlineUserDefinedTableFunction),
    DropFunction {
        function: ObjectName,
        if_exists: bool,
        is_temporary: bool,
    },
    DropTable {
        table: ObjectName,
        if_exists: bool,
        purge: bool,
    },
    CreateTemporaryView {
        view: ObjectName,
        #[serde(flatten)]
        definition: TemporaryViewDefinition,
    },
    DropView {
        view: ObjectName,
        if_exists: bool,
    },
    Write(Write),
    Explain {
        // TODO: Support stringified_plans
        mode: ExplainMode,
        input: Box<QueryPlan>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ReadType {
    NamedTable(ReadNamedTable),
    Udtf(ReadUdtf),
    DataSource(ReadDataSource),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadNamedTable {
    pub name: ObjectName,
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadUdtf {
    pub name: ObjectName,
    pub arguments: Vec<Expr>,
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadDataSource {
    pub format: Option<String>,
    pub schema: Option<Schema>,
    pub options: HashMap<String, String>,
    pub paths: Vec<String>,
    pub predicates: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Join {
    pub left: Box<QueryPlan>,
    pub right: Box<QueryPlan>,
    pub join_condition: Option<Expr>,
    pub join_type: JoinType,
    pub using_columns: Vec<Identifier>,
    pub join_data_type: Option<JoinDataType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetOperation {
    pub left: Box<QueryPlan>,
    pub right: Box<QueryPlan>,
    pub set_op_type: SetOpType,
    pub is_all: bool,
    pub by_name: bool,
    pub allow_missing_columns: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Aggregate {
    pub input: Box<QueryPlan>,
    pub group_type: GroupType,
    pub grouping_expressions: Vec<Expr>,
    pub aggregate_expressions: Vec<Expr>,
    pub pivot: Option<Pivot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    pub input: Box<QueryPlan>,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub with_replacement: bool,
    pub seed: Option<i64>,
    pub deterministic_order: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deduplicate {
    pub input: Box<QueryPlan>,
    pub column_names: Vec<Identifier>,
    pub all_columns_as_keys: bool,
    pub within_watermark: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    pub start: Option<i64>,
    pub end: i64,
    pub step: i64,
    pub num_partitions: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowString {
    pub input: Box<QueryPlan>,
    pub num_rows: usize,
    pub truncate: usize,
    pub vertical: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unpivot {
    pub input: Box<QueryPlan>,
    pub ids: Vec<Expr>,
    pub values: Vec<Expr>,
    pub variable_column_name: Identifier,
    pub value_column_name: Identifier,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parse {
    pub input: Box<QueryPlan>,
    pub format: ParseFormat,
    pub schema: Option<Schema>,
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupMap {
    pub input: Box<QueryPlan>,
    pub grouping_expressions: Vec<Expr>,
    pub function: CommonInlineUserDefinedFunction,
    pub sorting_expressions: Vec<Expr>,
    pub initial_input: Option<Box<QueryPlan>>,
    pub initial_grouping_expressions: Vec<Expr>,
    pub is_map_groups_with_state: Option<bool>, // TODO: this should probably be an enum
    pub output_mode: Option<String>,            // TODO: this should probably be an enum
    pub timeout_conf: Option<String>,           // TODO: this should probably be a struct
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoGroupMap {
    pub input: Box<QueryPlan>,
    pub input_grouping_expressions: Vec<Expr>,
    pub other: Box<QueryPlan>,
    pub other_grouping_expressions: Vec<Expr>,
    pub function: CommonInlineUserDefinedFunction,
    pub input_sorting_expressions: Vec<Expr>,
    pub other_sorting_expressions: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WithWatermark {
    pub input: Box<QueryPlan>,
    pub event_time: String,
    pub delay_threshold: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyInPandasWithState {
    pub input: Box<QueryPlan>,
    pub grouping_expressions: Vec<Expr>,
    pub function: CommonInlineUserDefinedFunction,
    pub output_schema: Schema,
    pub state_schema: Schema,
    pub output_mode: String,  // TODO: this should probably be an enum
    pub timeout_conf: String, // TODO: this should probably be a struct
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HtmlString {
    pub input: Box<QueryPlan>,
    pub num_rows: usize,
    pub truncate: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableDefinition {
    pub schema: Schema,
    pub comment: Option<String>,
    pub column_defaults: Vec<(String, Expr)>,
    pub constraints: Vec<TableConstraint>,
    pub location: Option<String>,
    pub file_format: Option<String>,
    pub table_partition_cols: Vec<Identifier>,
    pub file_sort_order: Vec<Vec<Expr>>,
    pub if_not_exists: bool,
    pub or_replace: bool,
    pub unbounded: bool,
    pub options: HashMap<String, String>,
    /// The query for `CREATE TABLE ... AS SELECT ...` (CTAS) statements.
    pub query: Option<Box<QueryPlan>>,
    pub definition: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseDefinition {
    pub if_not_exists: bool,
    pub comment: Option<String>,
    pub location: Option<String>,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogDefinition {
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemporaryViewDefinition {
    pub input: Box<QueryPlan>,
    pub is_global: bool,
    pub replace: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Write {
    pub input: Box<QueryPlan>,
    pub source: Option<String>, // TODO: is this the same as "provider" in `WriteOperationV2`?
    pub save_type: SaveType,
    pub mode: SaveMode,
    pub sort_columns: Vec<Identifier>,
    pub partitioning_columns: Vec<Identifier>,
    pub bucket_by: Option<SaveBucketBy>,
    pub options: HashMap<String, String>,
    pub table_properties: HashMap<String, String>,
    pub overwrite_condition: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SaveType {
    Path(String),
    Table {
        table: ObjectName,
        save_method: TableSaveMethod,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TableSaveMethod {
    SaveAsTable,
    InsertInto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveBucketBy {
    pub bucket_column_names: Vec<Identifier>,
    pub num_buckets: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SaveMode {
    // TODO: understand the difference between save modes in
    //  `WriteOperation` and `WriteOperationV2`
    Append,
    Create,
    CreateOrReplace,
    ErrorIfExists,
    Ignore,
    Overwrite,
    OverwritePartitions,
    Replace,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JoinType {
    Inner,
    FullOuter,
    LeftOuter,
    RightOuter,
    LeftAnti,
    LeftSemi,
    Cross,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinDataType {
    pub is_left_struct: bool,
    pub is_right_struct: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SetOpType {
    Intersect,
    Union,
    Except,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GroupType {
    GroupBy,
    Rollup,
    Cube,
    Pivot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ParseFormat {
    Unspecified,
    Csv,
    Json,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fraction {
    pub stratum: Literal,
    pub fraction: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pivot {
    pub column: Expr,
    pub values: Vec<Literal>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Replacement {
    pub old_value: Literal,
    pub new_value: Literal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageLevel {
    pub use_disk: bool,
    pub use_memory: bool,
    pub use_off_heap: bool,
    pub deserialized: bool,
    pub replication: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TableConstraint {
    Unique {
        name: Option<Identifier>,
        columns: Vec<Identifier>,
    },
    PrimaryKey {
        name: Option<Identifier>,
        columns: Vec<Identifier>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExplainMode {
    Unspecified,
    Analyze,
    Verbose,
    Extended,
    Codegen,
    Cost,
    Formatted,
}
