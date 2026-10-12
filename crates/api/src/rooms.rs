use axum::{
    Extension,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AppState,
    error::{ApiError, Json},
    session::Session,
    token,
};

struct RoomRow {
    id: Uuid,
    title: String,
    mode: String,
    status: String,
    accepting_joins: bool,
    game_state: Option<String>,
    share_token: String,
    created_at: DateTime<Utc>,
    closed_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct Room {
    id: Uuid,
    title: String,
    mode: String,
    status: String,
    accepting_joins: bool,
    game_state: Option<String>,
    share_url: String,
    #[serde(serialize_with = "crate::time::ms")]
    created_at: DateTime<Utc>,
    #[serde(serialize_with = "crate::time::ms_opt")]
    closed_at: Option<DateTime<Utc>>,
}

impl RoomRow {
    fn into_room(self, site_origin: &str) -> Room {
        Room {
            id: self.id,
            title: self.title,
            mode: self.mode,
            status: self.status,
            accepting_joins: self.accepting_joins,
            game_state: self.game_state,
            share_url: format!("{site_origin}/join/{}", self.share_token),
            created_at: self.created_at,
            closed_at: self.closed_at,
        }
    }
}

#[derive(Deserialize)]
pub struct CreateRoom {
    title: String,
    mode: String,
}

pub async fn create(
    State(state): State<AppState>,
    Extension(session): Extension<Session>,
    Json(body): Json<CreateRoom>,
) -> Result<(StatusCode, axum::Json<Room>), ApiError> {
    let title = body.title.trim();
    if title.is_empty() || title.chars().count() > 200 {
        return Err(ApiError::field(
            "title",
            "invalid",
            "Use 1 to 200 characters",
        ));
    }
    let game_state = match body.mode.as_str() {
        "assignment" => None,
        "game" => Some("lobby"),
        _ => {
            return Err(ApiError::field(
                "mode",
                "invalid",
                "Use \"assignment\" or \"game\"",
            ));
        }
    };
    let row = sqlx::query_as!(
        RoomRow,
        "INSERT INTO rooms (id, professor_id, title, mode, share_token, game_state)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, title, mode, status, accepting_joins, game_state, share_token,
                   created_at, closed_at",
        Uuid::now_v7(),
        session.professor_id,
        title,
        body.mode,
        token::random(18),
        game_state,
    )
    .fetch_one(&state.pool)
    .await?;
    Ok((
        StatusCode::CREATED,
        axum::Json(row.into_room(&state.site_origin)),
    ))
}

#[derive(Deserialize)]
pub struct ListParams {
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Serialize)]
pub struct RoomList {
    items: Vec<Room>,
    next_cursor: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    Extension(session): Extension<Session>,
    params: Result<Query<ListParams>, QueryRejection>,
) -> Result<axum::Json<RoomList>, ApiError> {
    let Query(params) = params?;
    let limit = params.limit.unwrap_or(50);
    if !(1..=200).contains(&limit) {
        return Err(ApiError::field("limit", "out_of_range", "Use 1 to 200"));
    }
    let cursor: Option<Uuid> = match params.cursor.as_deref() {
        None | Some("") => None,
        Some(c) => Some(
            c.parse()
                .map_err(|_| ApiError::field("cursor", "invalid", "Unknown cursor"))?,
        ),
    };
    let mut rows = sqlx::query_as!(
        RoomRow,
        "SELECT id, title, mode, status, accepting_joins, game_state, share_token,
                created_at, closed_at
           FROM rooms
          WHERE professor_id = $1
            AND ($2::uuid IS NULL OR (created_at, id) <
                 (SELECT created_at, id FROM rooms WHERE id = $2 AND professor_id = $1))
          ORDER BY created_at DESC, id DESC
          LIMIT $3",
        session.professor_id,
        cursor,
        limit + 1,
    )
    .fetch_all(&state.pool)
    .await?;

    let next_cursor = if rows.len() as i64 > limit {
        rows.truncate(limit as usize);
        rows.last().map(|r| r.id.to_string())
    } else {
        None
    };
    let items = rows
        .into_iter()
        .map(|r| r.into_room(&state.site_origin))
        .collect();
    Ok(axum::Json(RoomList { items, next_cursor }))
}

pub async fn get(
    State(state): State<AppState>,
    Extension(session): Extension<Session>,
    Path(rid): Path<String>,
) -> Result<axum::Json<Room>, ApiError> {
    let rid: Uuid = rid.parse().map_err(|_| ApiError::NotFound)?;
    let row = sqlx::query_as!(
        RoomRow,
        "SELECT id, title, mode, status, accepting_joins, game_state, share_token,
                created_at, closed_at
           FROM rooms WHERE id = $1 AND professor_id = $2",
        rid,
        session.professor_id,
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    Ok(axum::Json(row.into_room(&state.site_origin)))
}
