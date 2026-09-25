//! Length checks precede allocation. Scene publication additionally validates
//! semantic references, geometry, aggregate counts and application-owned assets.
use super::{DecodeError, Decoder};
use crate::{canvas::*, canvas_scene::*};
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn canvas_request(
        &mut self,
    ) -> Result<crate::canvas_resource::Request, DecodeError> {
        use crate::canvas_resource::{MAX_CHUNK_BYTES, Request, Update};
        Ok(match self.tag()? {
            0 => Request::Create,
            1 => Request::Begin(Update {
                id: self.resource()?,
                base: self.int()?,
                revision: self.int()?,
                generation: self.int()?,
                bytes: self.int()?,
            }),
            2 => {
                let id = self.resource()?;
                let revision = self.int()?;
                let offset = self.int()?;
                let payload = self.extension_payload(MAX_CHUNK_BYTES)?;
                Request::Chunk(
                    id,
                    revision,
                    offset,
                    crate::asset::Chunk::new(payload.0).map_err(|_| DecodeError::LimitExceeded)?,
                )
            }
            3 => Request::Publish(self.resource()?, self.int()?),
            4 => Request::Abort(self.resource()?, self.int()?),
            5 => Request::Release(self.resource()?),
            _ => return Err(DecodeError::Malformed),
        })
    }

    fn canvas_point(&mut self) -> Result<Point, DecodeError> {
        Ok(Point {
            x: self.float()?,
            y: self.float()?,
        })
    }
    fn canvas_rect(&mut self) -> Result<Rect, DecodeError> {
        Ok(Rect {
            x: self.float()?,
            y: self.float()?,
            width: self.float()?,
            height: self.float()?,
        })
    }
    fn canvas_transform(&mut self) -> Result<Transform, DecodeError> {
        Ok(Transform {
            a: self.float()?,
            b: self.float()?,
            c: self.float()?,
            d: self.float()?,
            tx: self.float()?,
            ty: self.float()?,
        })
    }
    fn canvas_key(&mut self) -> Result<ResourceKey, DecodeError> {
        Ok(ResourceKey {
            id: self.int()?,
            generation: self.int()?,
        })
    }
    fn canvas_path(&mut self, remaining: &mut usize) -> Result<Path, DecodeError> {
        let count = self.count(crate::canvas::MAX_PATH_COMMANDS.min(*remaining))?;
        *remaining -= count;
        let mut commands = Vec::with_capacity(count);
        for _ in 0..count {
            commands.push(match self.tag()? {
                0 => PathCommand::Move(self.canvas_point()?),
                1 => PathCommand::Line(self.canvas_point()?),
                2 => PathCommand::Quadratic(self.canvas_point()?, self.canvas_point()?),
                3 => PathCommand::Cubic(
                    self.canvas_point()?,
                    self.canvas_point()?,
                    self.canvas_point()?,
                ),
                4 => PathCommand::Close,
                _ => return Err(DecodeError::Malformed),
            });
        }
        Ok(Path(commands))
    }
    fn canvas_text(
        &mut self,
        maximum: usize,
        remaining: &mut usize,
    ) -> Result<String, DecodeError> {
        let text = self.bounded_text(maximum.min(*remaining))?;
        *remaining -= text.len();
        Ok(text)
    }
    fn canvas_hit_region(&mut self) -> Result<HitRegion, DecodeError> {
        Ok(match self.tag()? {
            0 => HitRegion::Rectangle(self.canvas_rect()?),
            1 => HitRegion::Ellipse(self.canvas_rect()?),
            2 => {
                let count = self.count(256)?;
                HitRegion::Polygon(
                    (0..count)
                        .map(|_| self.canvas_point())
                        .collect::<Result<_, _>>()?,
                )
            }
            _ => return Err(DecodeError::Malformed),
        })
    }
    fn canvas_paint(&mut self) -> Result<Paint, DecodeError> {
        Ok(Paint {
            fill: self.option(Self::int)?,
            stroke: self.option(|d| {
                Ok(Stroke {
                    color: d.int()?,
                    width: d.float()?,
                })
            })?,
        })
    }
    fn canvas_drawing(&mut self) -> Result<Drawing, DecodeError> {
        Ok(match self.tag()? {
            0 => {
                let shape = match self.tag()? {
                    0 => Shape::Rectangle(self.canvas_rect()?),
                    1 => Shape::Ellipse(self.canvas_rect()?),
                    2 => Shape::Path(self.canvas_key()?),
                    _ => return Err(DecodeError::Malformed),
                };
                Drawing::Shape(shape, self.canvas_paint()?)
            }
            1 => Drawing::Text(self.canvas_key()?, self.canvas_point()?, self.int()?),
            2 => Drawing::Image(self.canvas_key()?, self.canvas_rect()?),
            _ => return Err(DecodeError::Malformed),
        })
    }
}

pub fn decode_canvas_scene(bytes: &[u8]) -> Result<Scene, DecodeError> {
    if bytes.len() > MAX_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let version = decoder.int()?;
    if version != 1 {
        return Err(DecodeError::Malformed);
    }
    let mut text_remaining = MAX_TEXT_BYTES;
    let mut path_remaining = crate::canvas_scene::MAX_PATH_COMMANDS;
    let description = decoder.canvas_text(4096, &mut text_remaining)?;
    let count = decoder.count(MAX_RESOURCES)?;
    let mut resources = Vec::with_capacity(count);
    for _ in 0..count {
        let key = decoder.canvas_key()?;
        let data = match decoder.tag()? {
            0 => ResourceData::Path(decoder.canvas_path(&mut path_remaining)?),
            1 => ResourceData::Text(Text {
                value: decoder.canvas_text(16_384, &mut text_remaining)?,
                font_family: decoder.canvas_text(128, &mut text_remaining)?,
                font_size: decoder.float()?,
                font_weight: decoder.int()?,
            }),
            2 => ResourceData::Image(decoder.resource()?),
            _ => return Err(DecodeError::Malformed),
        };
        resources.push(Resource { key, data });
    }
    let count = decoder.count(MAX_ITEMS)?;
    let mut items = Vec::with_capacity(count);
    let mut interactive = 0;
    for _ in 0..count {
        let id = decoder.int()?;
        let transform = decoder.canvas_transform()?;
        let clips = decoder.count(MAX_CLIPS)?;
        let clips = (0..clips)
            .map(|_| decoder.canvas_rect())
            .collect::<Result<Vec<_>, _>>()?;
        let drawing = decoder.canvas_drawing()?;
        let interaction = decoder.option(|d| {
            interactive += 1;
            if interactive > MAX_INTERACTIVE_ITEMS {
                return Err(DecodeError::LimitExceeded);
            }
            Ok(Interaction {
                label: d.canvas_text(1024, &mut text_remaining)?,
                hit_region: d.canvas_hit_region()?,
                draggable: d.boolean()?,
                activatable: d.boolean()?,
            })
        })?;
        items.push(Item {
            id,
            transform,
            clips,
            drawing,
            interaction,
        });
    }
    if decoder.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    let scene = Scene {
        version,
        description,
        resources,
        items,
    };
    scene.validate().map_err(|error| match error {
        ValidationError::LimitExceeded => DecodeError::LimitExceeded,
        _ => DecodeError::Malformed,
    })?;
    Ok(scene)
}
