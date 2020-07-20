use std::io;

use bytes::{Buf, Bytes};
use futures::stream::StreamExt;
use warp::multipart::Part;

/// Collects raw data from [`Part`].
pub fn part_as_stream(raw: Part) -> impl futures::Stream<Item = Result<Bytes, io::Error>> {
    raw.stream().map(|r| {
        r.map(|mut x| x.to_bytes())
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "could not retrieve chunk"))
    })
}
