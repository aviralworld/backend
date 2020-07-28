use std::io;

use bytes::{Buf, Bytes};
use futures::stream::StreamExt;
use warp::multipart::Part;

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
