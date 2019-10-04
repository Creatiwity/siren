use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::Text;
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "Text"]
pub enum GroupType {
    UnitesLegales,
    Etablissements,
}

// SQL conversion
impl ToSql<Text, Pg> for GroupType {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        match *self {
            GroupType::UnitesLegales => out.write_all(b"unites_legales")?,
            GroupType::Etablissements => out.write_all(b"etablissements")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Pg> for GroupType {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        match not_none!(bytes) {
            b"unites_legales" => Ok(GroupType::UnitesLegales),
            b"etablissements" => Ok(GroupType::Etablissements),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}
