use std::sync::Arc;

use axum::{
    extract::{
        Path, 
        Query, 
        State
    },
    http::StatusCode,
    response::IntoResponse,
    Json
};

use serde_json::json;

use crate::{
    model::NoteModel,
    schema::{
        CreateNoteSchema,
        FilterOptions,
        UpdateNoteSchema
    },
    AppState
};

/** 
 *  GET: /api/notes
 * 
 *  note_list_handler : A handler function to fetch notes list from the database.
 *
 *  @param opts       : Optional parameters. These contains the Query filter options.
 *  @param State(data): Reference to the AppState of the application.
 *
 *  @return Result<impl IntoResponse,(StatusCode, Json)> : Returns Result with either Ok(Json) or Err(StatusCode, Json).
 */
pub async fn note_list_handler(
    opts: Option<Query<FilterOptions>>,                                             
    State(data): State<Arc<AppState>>                                            
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {             
    
    // Gets the filter options
    let Query(opts) = opts.unwrap_or_default();

    let limit = opts.limit.unwrap_or(10);
    let offset = (opts.page.unwrap_or(1) - 1) * limit;

    // Query the DB using SQLx
    let query_result = sqlx::query_as!(
        NoteModel,
        "SELECT * FROM notes ORDER BY id LIMIT $1 OFFSET $2",
        limit as i32,
        offset as i32
    )
    .fetch_all(&data.db)
    .await;

    // Error Response
    if query_result.is_err() {
        let error_response = serde_json::json!({
            "status": "fail",
            "message" : "Something bad happended while fetching all note items",
        });
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)));
    }

    let notes = query_result.unwrap();

    // Success Response
    let json_response = serde_json::json!({
        "status": "success",
        "results": notes.len(),
        "notes": notes
    });
    Ok(Json(json_response))
}

/**
 * POST: /api/notes
 * 
 * create_note_handler: A handler function to create a note in the database.
 * 
 * @param State(data) : Reference to the AppState of the application.
 * @param Json(body)  : JSON payload of the request
 * 
 * @return Result<impl IntoResponse, (StatusCode, Json)> : Returns Result with either Ok(Json) or Err(StatusCode, Json).
 */
pub async fn create_note_handler(
    State(data): State<Arc<AppState>>,
    Json(body): Json<CreateNoteSchema>
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {

    // Query the DB to insert a row with NoteModel members and with title, content and catergory values
    let query_result = sqlx::query_as!(
        NoteModel,
        "INSERT INTO notes (title, content, category) VALUES ($1, $2, $3) RETURNING *",
        body.title.to_string(),
        body.content.to_string(),
        body.category.to_owned().unwrap_or("".to_string())
    )
    .fetch_all(&data.db)
    .await;

    match query_result {

        // Success Response
        Ok(note) => {
            let note_response = json!({
                "status": "success",
                "data": json!({
                    "note": note
                })
            });

            return Ok((StatusCode::CREATED, Json(note_response)));
        }

        // Error Response
        Err(e) => {
            // Duplicate title error
            if e.to_string()
                .contains("duplicate key value violates unique constraint")
            {
                let error_response = serde_json::json!({
                    "status": "fail",
                    "message": "Note with that title already exists"
                });
                return Err((StatusCode::CONFLICT, Json(error_response)));
            }
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "error",
                    "message": format!("{:?}", e)
                }))
            ));
        }
    }
}

/**
 * GET: api/notes/:id
 * 
 * get_note_handler  : Handler function to fetch a note/row from the DB
 * 
 * @param Path(id)   : The id parameter from the request url, expected to be Uuid. Path extractor extracts the 'id'.
 * @param State(data): The reference to AppState of the Application
 * 
 * @return Result<impl IntoResponse, (StatusCode, Json)> : Returns Result with either Ok(Json) or Err(StatusCode, Json).
 */
pub async fn get_note_handler(
    Path(id): Path<uuid::Uuid>,
    State(data): State<Arc<AppState>>
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {

    // Query the DB to fetch a single row
    let query_result = sqlx::query_as!(
        NoteModel,
        "SELECT * FROM notes WHERE id = $1", id
    )
    .fetch_all(&data.db)
    .await;

    
    match query_result {
        // Success Response
        Ok(note) => {
            let note_response = serde_json::json!({
                "status": "success",
                "data": serde_json::json!({
                    "note": note
                })
            });
            return Ok(Json(note_response));
        }
        // Error Response
        Err(_) => {
            let error_response = serde_json::json!({
                "status": "fail",
                "message": format!("Note with ID: {} not found", id)
            });
            return Err((StatusCode::NOT_FOUND, Json(error_response)));
        }
    }
}

/**
 * PATCH: api/notes/:id
 * 
 * edit_note_handler: Handler function to modify a note/row from the DB given by the id
 * 
 * @param Path(id)   : The id parameter from the request url, expected to be Uuid. Path extractor extracts the 'id'.
 * @param State(data): The reference to AppState of the Application
 * @param Json(body) : JSON payload containing the updated values of the note fields
 * 
 * @return Result<impl IntoResponse, (StatusCode, Json)> : Returns Result with either Ok(Json) or Err(StatusCode, Json).
 */
pub async fn edit_note_handler(
    Path(id): Path<uuid::Uuid>,
    State(data): State<Arc<AppState>>,
    Json(body): Json<UpdateNoteSchema>
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {

    // Query and check if the row exists with the id in the DB
    let query_result = sqlx::query_as!(
        NoteModel,
        "SELECT * FROM notes WHERE id = $1", id
    )
    .fetch_one(&data.db)
    .await;

    // If not, raise error response
    if query_result.is_err() {
        let error_response = serde_json::json!({
            "status": "fail",
            "message": format!("Note with ID: {} not found", id)
        });
        return Err((StatusCode::NOT_FOUND, Json(error_response)));
    }

    // Chrono for updating time to current
    let now = chrono::Utc::now();
    // Fetched note
    let note = query_result.unwrap();

    // Query to modify the row data
    let query_result = sqlx::query_as!(
        NoteModel,
        "UPDATE notes SET title = $1, content = $2, category = $3, published = $4, updated_at = $5 WHERE id = $6 RETURNING *",
        body.title.to_owned().unwrap_or(note.title),
        body.content.to_owned().unwrap_or(note.content),
        body.category.to_owned().unwrap_or(note.category.unwrap()),
        body.published.unwrap_or(note.published.unwrap()),
        now,
        id
    )
    .fetch_one(&data.db)
    .await;

    match query_result {
        // Success Response
        Ok(note) => {
            let note_response = serde_json::json!({
                "status": "success",
                "data": serde_json::json!({
                    "note": note
                })
            });
            return Ok(Json(note_response));
        }
        // Error Response
        Err(err) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "error",
                    "message": format!("{:?}", err)
                }))
            ));
        }
    }
}

/**
 * DELETE: api/notes/:id
 * 
 * delete_note_handler: Handler function to delete a note/row from the DB given by the id
 * 
 * @param Path(id)   : The id parameter from the request url, expected to be Uuid. Path extractor extracts the 'id'.
 * @param State(data): The reference to AppState of the Application
 * 
 * @return Result<impl IntoResponse, (StatusCode, Json)> : Returns Result with either Ok(Json) or Err(StatusCode, Json).
 */
pub async fn delete_note_handler(
    Path(id): Path<uuid::Uuid>,
    State(data): State<Arc<AppState>>
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {

    // Query to delete row
    let rows_affected = sqlx::query!("DELETE FROM notes WHERE id = $1", id)
        .execute(&data.db)
        .await
        .unwrap()
        .rows_affected();

    // Error: No rows found to delete
    if rows_affected == 0 {
        let error_response = serde_json::json!({
            "status": "fail",
            "message": format!("Note with ID: {} not found", id)
        });
        return Err((StatusCode::NOT_FOUND, Json(error_response)));
    }
    Ok(StatusCode::NO_CONTENT)
}

// Handler for basic endpoint: /api/healthchecker
pub async fn health_checker_handler() -> impl IntoResponse {
    const MESSAGE: &str = "Simple CRUD API with Rust, SQLx, Postgres and Axum";

    let json_response = serde_json::json!({
        "status": "success",
        "message": MESSAGE
    });

    Json(json_response)
}
