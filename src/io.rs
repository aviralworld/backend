use std::io;

use bytes::{Buf, Bytes};
use futures::stream::StreamExt;
use warp::multipart::{FormData, Part};

use crate::errors::BackendError;

pub struct Upload {
    pub(crate) audio: Part,
    pub(crate) metadata: Part,
}

pub async fn parse_upload(content: FormData) -> Result<Upload, BackendError> {
    let mut parts = collect_parts(content).await?;
    let upload = parse_parts(&mut parts)?;

    Ok(upload)
}

pub async fn collect_parts(content: FormData) -> Result<Vec<Part>, BackendError> {
    let parts = (content.collect::<Vec<Result<Part, _>>>()).await;
    let vec = parts
        .into_iter()
        .collect::<Result<Vec<Part>, _>>()
        // TODO this should be a more specific error
        .map_err(|_| BackendError::BadRequest)?;
    Ok(vec)
}

pub fn parse_parts(parts: &mut Vec<Part>) -> Result<Upload, BackendError> {
    let mut audio = None;
    let mut metadata = None;

    for p in parts.drain(0..) {
        let name = p.name().to_owned();

        if name == "audio" {
            audio = Some(p);
        } else if name == "metadata" {
            metadata = Some(p);
        }
    }

    if metadata.is_none() || audio.is_none() {
        return Err(BackendError::PartsMissing);
    }

    Ok(Upload {
        audio: audio.unwrap(),
        metadata: metadata.unwrap(),
    })
}

/// Collects chunks of [`Part`].
pub async fn part_as_vec(raw: Part) -> Result<Vec<u8>, ()> {
    let vec_of_results = part_as_stream(raw).collect::<Vec<_>>().await;

    let vec_of_vecs = vec_of_results
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ())?;

    Ok(vec_of_vecs.concat())
}

/// Collects raw data from [`Part`].
pub fn part_as_stream(raw: Part) -> impl futures::Stream<Item = Result<Bytes, io::Error>> {
    raw.stream().map(|r| {
        r.map(|mut x| x.to_bytes())
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "could not retrieve chunk"))
    })
}
