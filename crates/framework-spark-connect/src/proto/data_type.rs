use crate::error::{ProtoFieldExt, SparkError, SparkResult};
use crate::proto::data_type_json::parse_spark_json_data_type;
use crate::spark::connect::{data_type as sdt, DataType};
use framework_common::spec;
use framework_sql::data_type::parse_spark_data_type;
use std::collections::HashMap;

pub(crate) const DEFAULT_FIELD_NAME: &str = "value";

#[allow(dead_code)]
pub(crate) const SPARK_DECIMAL_MAX_PRECISION: u8 = 38;
#[allow(dead_code)]
pub(crate) const SPARK_DECIMAL_MAX_SCALE: i8 = 38;
pub(crate) const SPARK_DECIMAL_USER_DEFAULT_PRECISION: u8 = 10;
pub(crate) const SPARK_DECIMAL_USER_DEFAULT_SCALE: i8 = 0;
#[allow(dead_code)]
pub(crate) const SPARK_DECIMAL_SYSTEM_DEFAULT_PRECISION: u8 = 38;
#[allow(dead_code)]
pub(crate) const SPARK_DECIMAL_SYSTEM_DEFAULT_SCALE: i8 = 18;

pub(crate) fn parse_spark_schema(schema: &str) -> SparkResult<spec::DataType> {
    // TODO: Replicate parseDatatypeString functionality exactly
    if let Ok(dt) = parse_spark_data_type(schema) {
        Ok(dt)
    } else if let Ok(dt) = parse_spark_data_type(format!("struct<{schema}>").as_str()) {
        Ok(dt)
    } else {
        parse_spark_json_data_type(schema)?.try_into()
    }
}

impl TryFrom<sdt::StructField> for spec::Field {
    type Error = SparkError;

    fn try_from(field: sdt::StructField) -> SparkResult<spec::Field> {
        let sdt::StructField {
            name,
            data_type,
            nullable,
            metadata,
        } = field;
        let data_type = data_type.required("data type")?;
        let data_type = spec::DataType::try_from(data_type)?;
        let metadata: HashMap<String, String> = metadata
            .map(|m| -> SparkResult<_> { Ok(serde_json::from_str(m.as_str())?) })
            .transpose()?
            .unwrap_or_default();
        Ok(spec::Field {
            name,
            data_type,
            nullable,
            metadata,
        })
    }
}

impl TryFrom<DataType> for spec::DataType {
    type Error = SparkError;

    fn try_from(data_type: DataType) -> SparkResult<spec::DataType> {
        use crate::spark::connect::data_type::Kind;

        let DataType { kind } = data_type;
        let kind = kind.required("data type kind")?;
        match kind {
            Kind::Null(_) => Ok(spec::DataType::Null),
            Kind::Binary(_) => Ok(spec::DataType::Binary),
            Kind::Boolean(_) => Ok(spec::DataType::Boolean),
            Kind::Byte(_) => Ok(spec::DataType::Byte),
            Kind::Short(_) => Ok(spec::DataType::Short),
            Kind::Integer(_) => Ok(spec::DataType::Integer),
            Kind::Long(_) => Ok(spec::DataType::Long),
            Kind::Float(_) => Ok(spec::DataType::Float),
            Kind::Double(_) => Ok(spec::DataType::Double),
            Kind::Decimal(sdt::Decimal {
                scale,
                precision,
                type_variation_reference: _,
            }) => {
                let scale = scale
                    .map(i8::try_from)
                    .transpose()
                    .map_err(|_| SparkError::invalid("decimal scale"))?
                    .unwrap_or(SPARK_DECIMAL_USER_DEFAULT_SCALE);
                let precision = precision
                    .map(u8::try_from)
                    .transpose()
                    .map_err(|_| SparkError::invalid("decimal precision"))?
                    .unwrap_or(SPARK_DECIMAL_USER_DEFAULT_PRECISION);
                Ok(spec::DataType::Decimal { scale, precision })
            }
            Kind::String(_) => Ok(spec::DataType::String),
            Kind::Char(sdt::Char {
                length,
                type_variation_reference: _,
            }) => {
                let length = length
                    .try_into()
                    .map_err(|_| SparkError::invalid("char length"))?;
                Ok(spec::DataType::Char { length })
            }
            Kind::VarChar(sdt::VarChar {
                length,
                type_variation_reference: _,
            }) => {
                let length = length
                    .try_into()
                    .map_err(|_| SparkError::invalid("varchar length"))?;
                Ok(spec::DataType::VarChar { length })
            }
            Kind::Date(_) => Ok(spec::DataType::Date),
            Kind::Timestamp(_) => Ok(spec::DataType::Timestamp),
            Kind::TimestampNtz(_) => Ok(spec::DataType::TimestampNtz),
            Kind::CalendarInterval(_) => Ok(spec::DataType::CalendarInterval),
            Kind::YearMonthInterval(sdt::YearMonthInterval {
                start_field,
                end_field,
                type_variation_reference: _,
            }) => {
                let start_field = start_field
                    .map(|f| spec::YearMonthIntervalField::try_from(f))
                    .transpose()?;
                let end_field = end_field
                    .map(|f| spec::YearMonthIntervalField::try_from(f))
                    .transpose()?;
                Ok(spec::DataType::YearMonthInterval {
                    start_field,
                    end_field,
                })
            }
            Kind::DayTimeInterval(sdt::DayTimeInterval {
                start_field,
                end_field,
                type_variation_reference: _,
            }) => {
                let start_field = start_field
                    .map(|f| spec::DayTimeIntervalField::try_from(f))
                    .transpose()?;
                let end_field = end_field
                    .map(|f| spec::DayTimeIntervalField::try_from(f))
                    .transpose()?;
                Ok(spec::DataType::DayTimeInterval {
                    start_field,
                    end_field,
                })
            }
            Kind::Array(array) => {
                let sdt::Array {
                    element_type,
                    contains_null,
                    type_variation_reference: _,
                } = *array;
                let element_type = element_type.required("array element type")?;
                Ok(spec::DataType::Array {
                    element_type: Box::new(spec::DataType::try_from(*element_type)?),
                    contains_null,
                })
            }
            Kind::Struct(sdt::Struct {
                fields,
                type_variation_reference: _,
            }) => {
                let fields = fields
                    .into_iter()
                    .map(|f| spec::Field::try_from(f))
                    .collect::<SparkResult<_>>()?;
                Ok(spec::DataType::Struct {
                    fields: spec::Fields::new(fields),
                })
            }
            Kind::Map(map) => {
                let sdt::Map {
                    key_type,
                    value_type,
                    value_contains_null,
                    type_variation_reference: _,
                } = *map;
                let key_type = key_type.required("map key type")?;
                let value_type = value_type.required("map value type")?;
                Ok(spec::DataType::Map {
                    key_type: Box::new(spec::DataType::try_from(*key_type)?),
                    value_type: Box::new(spec::DataType::try_from(*value_type)?),
                    value_contains_null,
                })
            }
            Kind::Udt(udt) => {
                let sdt::Udt {
                    r#type: _,
                    jvm_class,
                    python_class,
                    serialized_python_class,
                    sql_type,
                } = *udt;
                let sql_type = sql_type.required("UDT sql type")?;
                Ok(spec::DataType::UserDefined {
                    jvm_class,
                    python_class,
                    serialized_python_class,
                    sql_type: Box::new(spec::DataType::try_from(*sql_type)?),
                })
            }
            Kind::Unparsed(sdt::Unparsed { data_type_string }) => {
                Ok(parse_spark_schema(data_type_string.as_str())?)
            }
        }
    }
}

impl TryFrom<spec::Field> for sdt::StructField {
    type Error = SparkError;

    fn try_from(field: spec::Field) -> SparkResult<sdt::StructField> {
        let spec::Field {
            name,
            data_type,
            nullable,
            metadata,
        } = field;
        let data_type = data_type.try_into()?;
        let metadata = serde_json::to_string(&metadata)?;
        Ok(sdt::StructField {
            name,
            data_type: Some(data_type),
            nullable,
            metadata: Some(metadata),
        })
    }
}

impl TryFrom<spec::DataType> for DataType {
    type Error = SparkError;

    fn try_from(data_type: spec::DataType) -> SparkResult<DataType> {
        use crate::spark::connect::data_type::Kind;

        let kind = match data_type {
            spec::DataType::Null => Kind::Null(sdt::Null::default()),
            spec::DataType::Binary => Kind::Binary(sdt::Binary::default()),
            spec::DataType::Boolean => Kind::Boolean(sdt::Boolean::default()),
            spec::DataType::Byte => Kind::Byte(sdt::Byte::default()),
            spec::DataType::Short => Kind::Short(sdt::Short::default()),
            spec::DataType::Integer => Kind::Integer(sdt::Integer::default()),
            spec::DataType::Long => Kind::Long(sdt::Long::default()),
            spec::DataType::Float => Kind::Float(sdt::Float::default()),
            spec::DataType::Double => Kind::Double(sdt::Double::default()),
            spec::DataType::Decimal { scale, precision } => Kind::Decimal(sdt::Decimal {
                scale: Some(scale as i32),
                precision: Some(precision as i32),
                type_variation_reference: 0,
            }),
            spec::DataType::String => Kind::String(sdt::String::default()),
            spec::DataType::Char { length } => {
                let length = length
                    .try_into()
                    .map_err(|_| SparkError::invalid("char length"))?;
                Kind::Char(sdt::Char {
                    length,
                    type_variation_reference: 0,
                })
            }
            spec::DataType::VarChar { length } => {
                let length = length
                    .try_into()
                    .map_err(|_| SparkError::invalid("varchar length"))?;
                Kind::VarChar(sdt::VarChar {
                    length,
                    type_variation_reference: 0,
                })
            }
            spec::DataType::Date => Kind::Date(sdt::Date::default()),
            spec::DataType::Timestamp => Kind::Timestamp(sdt::Timestamp::default()),
            spec::DataType::TimestampNtz => Kind::TimestampNtz(sdt::TimestampNtz::default()),
            spec::DataType::CalendarInterval => {
                Kind::CalendarInterval(sdt::CalendarInterval::default())
            }
            spec::DataType::YearMonthInterval {
                start_field,
                end_field,
            } => Kind::YearMonthInterval(sdt::YearMonthInterval {
                start_field: start_field.map(|f| f as i32),
                end_field: end_field.map(|f| f as i32),
                type_variation_reference: 0,
            }),
            spec::DataType::DayTimeInterval {
                start_field,
                end_field,
            } => Kind::DayTimeInterval(sdt::DayTimeInterval {
                start_field: start_field.map(|f| f as i32),
                end_field: end_field.map(|f| f as i32),
                type_variation_reference: 0,
            }),
            spec::DataType::Array {
                element_type,
                contains_null,
            } => Kind::Array(Box::new(sdt::Array {
                element_type: Some(Box::new((*element_type).try_into()?)),
                contains_null,
                type_variation_reference: 0,
            })),
            spec::DataType::Struct { fields } => {
                let fields: Vec<spec::Field> = fields.into();
                Kind::Struct(sdt::Struct {
                    fields: fields
                        .into_iter()
                        .map(|f| f.try_into())
                        .collect::<SparkResult<Vec<sdt::StructField>>>()?,
                    type_variation_reference: 0,
                })
            }
            spec::DataType::Map {
                key_type,
                value_type,
                value_contains_null,
            } => Kind::Map(Box::new(sdt::Map {
                key_type: Some(Box::new((*key_type).try_into()?)),
                value_type: Some(Box::new((*value_type).try_into()?)),
                value_contains_null,
                type_variation_reference: 0,
            })),
            spec::DataType::UserDefined {
                jvm_class,
                python_class,
                serialized_python_class,
                sql_type,
            } => Kind::Udt(Box::new(sdt::Udt {
                r#type: "udt".to_string(),
                jvm_class,
                python_class,
                serialized_python_class,
                sql_type: Some(Box::new((*sql_type).try_into()?)),
            })),
        };
        Ok(DataType { kind: Some(kind) })
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_spark_data_type, parse_spark_schema, DEFAULT_FIELD_NAME};
    use crate::error::{SparkError, SparkResult};
    use framework_common::tests::test_gold_set;

    #[test]
    fn test_parse_spark_data_type_gold_set() -> SparkResult<()> {
        test_gold_set(
            "tests/gold_data/data_type.json",
            |s: String| Ok(parse_spark_data_type(&s)?),
            |e: String| SparkError::internal(e),
        )
    }

    #[test]
    fn test_parse_spark_schema_gold_set() -> SparkResult<()> {
        test_gold_set(
            "tests/gold_data/table_schema.json",
            |s: String| Ok(parse_spark_schema(&s)?.into_schema(DEFAULT_FIELD_NAME, true)),
            |e: String| SparkError::internal(e),
        )
    }
}
