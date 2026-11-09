use std::collections::HashMap;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Deserializer, Serialize};

use super::error::DbError;

macro_rules! str_enum {
    ($name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        #[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }

        impl $name {
            pub fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $text),+ }
            }
        }

        impl std::str::FromStr for $name {
            type Err = DbError;
            fn from_str(s: &str) -> Result<Self, DbError> {
                match s {
                    $($text => Ok(Self::$variant),)+
                    other => Err(DbError::Invalid(format!("{} {other:?}", stringify!($name)))),
                }
            }
        }

        impl rusqlite::types::ToSql for $name {
            fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
                Ok(self.as_str().into())
            }
        }

        impl rusqlite::types::FromSql for $name {
            fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
                value.as_str()?.parse().map_err(|e| rusqlite::types::FromSqlError::Other(Box::new(e)))
            }
        }
    };
}

str_enum!(ItemType {
    Task => "task",
    Reminder => "reminder",
    Note => "note",
    Idea => "idea",
    ProjectRef => "project_ref",
    Event => "event",
});

str_enum!(Status {
    Inbox => "inbox",
    Next => "next",
    Doing => "doing",
    Done => "done",
});

str_enum!(Priority {
    Low => "low",
    Normal => "normal",
    High => "high",
    Urgent => "urgent",
});

str_enum!(BoardKind {
    Project => "project",
    Personal => "personal",
});

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub kind: ItemType,
    pub title: String,
    pub body: Option<String>,
    pub status: Status,
    pub priority: Priority,
    pub due_at: Option<DateTime<Utc>>,
    pub remind_at: Option<DateTime<Utc>>,
    pub snoozed_until: Option<DateTime<Utc>>,
    pub recurrence: Option<String>,
    pub project_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct NewItem {
    pub kind: ItemType,
    pub title: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub priority: Option<Priority>,
    #[serde(default)]
    pub due_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub remind_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub recurrence: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub board_id: Option<String>,
}

#[allow(clippy::derivable_impls)]
impl Default for ItemType {
    fn default() -> Self {
        ItemType::Task
    }
}

/// Deserializes a field so that an absent key is `None` (leave alone) while an
/// explicit `null` is `Some(None)` (clear the value).
fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// Partial update. `None` leaves the field alone; for nullable fields,
/// `Some(None)` clears it.
#[derive(Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ItemPatch {
    #[serde(default)]
    pub kind: Option<ItemType>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub body: Option<Option<String>>,
    #[serde(default)]
    pub status: Option<Status>,
    #[serde(default)]
    pub priority: Option<Priority>,
    #[serde(default, deserialize_with = "double_option")]
    pub due_at: Option<Option<DateTime<Utc>>>,
    #[serde(default, deserialize_with = "double_option")]
    pub remind_at: Option<Option<DateTime<Utc>>>,
    #[serde(default, deserialize_with = "double_option")]
    pub snoozed_until: Option<Option<DateTime<Utc>>>,
    #[serde(default, deserialize_with = "double_option")]
    pub recurrence: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub project_id: Option<Option<String>>,
}

/// Which items to list. `now`/`tz` are supplied by the caller (the command
/// layer injects the real clock; tests pin them).
#[derive(Clone, PartialEq, Debug)]
pub enum ItemFilter {
    Inbox,
    Today { now: DateTime<Utc>, tz: Tz },
    Upcoming { now: DateTime<Utc>, days: u32 },
    Ideas,
    Notes,
    Project(String),
    Board(String),
    All,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub inbox: u32,
    pub today: u32,
    pub upcoming: u32,
    pub ideas: u32,
    pub notes: u32,
    pub by_project: HashMap<String, u32>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub color: String,
    pub board_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    pub id: String,
    pub name: String,
    pub kind: BoardKind,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    pub item_id: String,
    pub board_id: String,
    pub x: f64,
    pub y: f64,
}

pub fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}
