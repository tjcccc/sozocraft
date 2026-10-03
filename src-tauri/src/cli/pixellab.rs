use super::{
    emit,
    output::{validate_output, Output},
    pixellab_arguments::PixelLab,
};
use crate::pixellab::{self, request, Client};
use serde_json::json;

pub(super) async fn execute(command: PixelLab) -> Result<(), String> {
    let (route, dry_run) = match &command {
        PixelLab::Balance { dry_run } => ("/balance".into(), *dry_run),
        PixelLab::Characters {
            limit,
            offset,
            dry_run,
        } => (
            format!("/characters?limit={limit}&offset={offset}"),
            *dry_run,
        ),
        PixelLab::Character { id, dry_run } => (format!("/characters/{id}"), *dry_run),
        PixelLab::Job { id, dry_run } => (format!("/background-jobs/{id}"), *dry_run),
        PixelLab::Submit {
            create, dry_run, ..
        } => (
            if *create {
                request::CREATE_ROUTE
            } else {
                request::ANIMATE_ROUTE
            }
            .into(),
            *dry_run,
        ),
        PixelLab::Download {
            id,
            output,
            dry_run,
        } => {
            validate_output(output, "zip")?;
            (format!("/characters/{id}/zip"), *dry_run)
        }
    };
    let payload = if let PixelLab::Submit {
        request: path,
        reference,
        create,
        ..
    } = &command
    {
        Some(request::load(path, reference.as_deref(), *create)?)
    } else {
        None
    };
    if dry_run {
        let mut options = payload.clone().unwrap_or_else(|| json!({}));
        // Dry runs never echo prompt text, image bytes or a token accidentally pasted into JSON.
        let key = std::env::var("PIXELLAB_API_KEY").ok();
        pixellab::sanitize(&mut options, key.as_deref());
        return emit(
            &json!({"status":"validated", "provider":"pixellab", "platform":"pixellab",
            "method": if payload.is_some() { "POST" } else { "GET" }, "route":route,
            "model": if route == request::CREATE_ROUTE { "v3" } else { payload.as_ref().map(request::mode).unwrap_or("none") },
            "options":options, "referenceCount":payload.as_ref().map_or(0, |v| usize::from(!v["reference_image"].is_null()))}),
        );
    }
    if let PixelLab::Download { id, output, .. } = &command {
        let output = Output::for_path(output.clone(), "zip")?;
        let bytes = Client::new(None)?.download_to(id, &output.partial).await?;
        output.publish()?;
        return emit(
            &json!({"status":"completed", "provider":"pixellab", "platform":"pixellab",
            "characterId":id,"outputPath":output.final_path,"bytes":bytes}),
        );
    }
    let client = Client::new(Some(pixellab::api_key()?))?;
    let result = match &command {
        PixelLab::Balance { .. } => client.balance().await?,
        PixelLab::Characters { limit, offset, .. } => client.characters(*limit, *offset).await?,
        PixelLab::Character { id, .. } => client.character(id).await?,
        PixelLab::Job { id, .. } => client.job(id).await?,
        PixelLab::Submit { create, .. } => {
            client
                .submit(
                    payload.as_ref().ok_or("Missing PixelLab request.")?,
                    *create,
                )
                .await?
        }
        PixelLab::Download { .. } => unreachable!(),
    };
    if let PixelLab::Submit { create, .. } = &command {
        // Preserve all per-direction jobs in the first event, before validating response shape.
        emit(
            &json!({"status":"submitted", "provider":"pixellab", "platform":"pixellab",
            "model":if *create { "v3" } else { request::mode(payload.as_ref().unwrap()) },
            "characterId":if *create { result["character_id"].clone() } else { payload.as_ref().unwrap()["character_id"].clone() },
            "operation":result["background_job_id"], "operations":result["background_job_ids"],
            "animationGroupId":result["animation_group_id"], "directions":result["directions"], "data":result}),
        )?;
        let ids: Vec<&str> = if *create {
            result["background_job_id"].as_str().into_iter().collect()
        } else {
            result["background_job_ids"]
                .as_array()
                .map(|v| v.iter().map(|v| v.as_str().unwrap_or_default()).collect())
                .unwrap_or_default()
        };
        if ids.is_empty() || ids.iter().any(|id| request::checked_id(id).is_err()) {
            return Err("PixelLab submission response has missing/invalid job IDs; inspect account before resubmitting.".into());
        }
        if result["status"] == "failed" {
            return Err(
                "PixelLab submission reports failure; inspect returned jobs before resubmitting."
                    .into(),
            );
        }
        return Ok(());
    }
    let status = if matches!(command, PixelLab::Job { .. }) {
        match result["status"].as_str() {
            Some("completed") => "completed",
            Some("failed") => "failed",
            _ => "pending",
        }
    } else {
        "completed"
    };
    let operation = match &command {
        PixelLab::Job { id, .. } => Some(id),
        _ => None,
    };
    emit(
        &json!({"status":status, "provider":"pixellab", "platform":"pixellab", "operation":operation, "data":result}),
    )?;
    if status == "failed" {
        return Err(
            "PixelLab background job failed; inspect character/account before resubmitting.".into(),
        );
    }
    Ok(())
}
