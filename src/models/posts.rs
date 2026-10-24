pub use super::_entities::posts::{ActiveModel, Entity, Model};
use sea_orm::entity::prelude::*;
pub type Posts = Entity;

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> std::result::Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if !insert && self.updated_at.is_unchanged() {
            let mut this = self;
            this.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now().into());
            Ok(this)
        } else {
            Ok(self)
        }
    }
}

// implement your read-oriented logic here
impl Model {}

// implement your write-oriented logic here
impl ActiveModel {}

// implement your custom finders, selectors oriented logic here
impl Entity {}

/// Domain rule (Q8, specs/due-dates.md): a due date must not be in the past
/// at write time; the UTC clock decides (Q9). Format validation (RD-10)
/// arrives with T-11 — unparseable values pass through for now.
pub fn ensure_due_date_is_future(raw: Option<&str>) -> Result<(), String> {
    let Some(raw) = raw else {
        return Ok(());
    };
    if let Ok(due) = chrono::DateTime::parse_from_rfc3339(raw) {
        if due < chrono::Utc::now() {
            return Err(format!("due date {raw} is in the past"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Model;
    use crate::dtos::posts::PostDto;

    fn sample_post() -> Model {
        Model {
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
            id: 1,
            title: "Sample".to_string(),
            published: Some(true),
            due_date: None,
        }
    }

    // Q8 (specs/due-dates.md, T-12): a due date in the past is rejected on
    // write; the UTC clock decides (Q9).
    #[test]
    fn past_due_date_is_rejected() {
        let past = (chrono::Utc::now() - chrono::Duration::days(1)).to_rfc3339();
        assert!(
            super::ensure_due_date_is_future(Some(&past)).is_err(),
            "a due date in the past must be rejected (Q8)"
        );
    }

    #[test]
    fn future_and_absent_due_dates_are_accepted() {
        let future = (chrono::Utc::now() + chrono::Duration::days(2)).to_rfc3339();
        assert!(super::ensure_due_date_is_future(Some(&future)).is_ok());
        assert!(super::ensure_due_date_is_future(None).is_ok());
    }

    // RD-2 (specs/due-dates.md): every post representation (PostDto on list
    // and detail) shall include `due_date` as an RFC 3339 timestamp or null.
    #[test]
    fn post_dto_exposes_due_date() {
        let dto = PostDto::from(sample_post());
        let json = serde_json::to_value(&dto).expect("PostDto must serialize");

        let due_date = json
            .get("due_date")
            .expect("PostDto must expose a due_date field (RD-2)");

        assert!(
            due_date.is_null()
                || due_date
                    .as_str()
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .is_some(),
            "due_date must be an RFC 3339 string or null (RD-2)"
        );
    }
}
