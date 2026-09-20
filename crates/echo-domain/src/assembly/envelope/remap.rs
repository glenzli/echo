//! Re-anchor a prepared-source curve when source edits change its time axis.
use super::{GainEnvelope, GainEnvelopePoint, SoundAssemblyError};
use crate::{EditSegmentState, EditTimeline};
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Span {
    original_start: u64,
    original_end: u64,
    output_start: u64,
}

fn spans(timeline: &EditTimeline) -> Vec<Span> {
    let mut output = 0;
    let mut result: Vec<Span> = Vec::new();
    for segment in timeline.segments() {
        if segment.state() != EditSegmentState::Hidden {
            let span = Span {
                original_start: segment.source_start_millis(),
                original_end: segment.source_end_millis(),
                output_start: output,
            };
            if let Some(previous) = result.last_mut()
                && previous.original_end == span.original_start
                && previous.output_start + previous.original_end - previous.original_start == output
            {
                previous.original_end = span.original_end;
            } else {
                result.push(span);
            }
            output += segment.source_end_millis() - segment.source_start_millis();
        }
        output += segment.gap_after_millis();
    }
    result
}

impl GainEnvelope {
    fn gain_at(&self, position: u64) -> i16 {
        let right = self
            .points
            .partition_point(|point| point.source_millis <= position);
        if right == 0 {
            return self.points[0].gain_centibels;
        }
        let left = self.points[right - 1];
        let Some(next) = self.points.get(right) else {
            return left.gain_centibels;
        };
        let delta = i128::from(next.gain_centibels) - i128::from(left.gain_centibels);
        let value = i128::from(left.gain_centibels)
            + delta * i128::from(position - left.source_millis)
                / i128::from(next.source_millis - left.source_millis);
        i16::try_from(value).expect("interpolation stays inside validated gain bounds")
    }

    /// Retains gain on surviving original audio after trimming or hiding source regions.
    /// Restored audio has unity gain; inserted silence does not carry gain evidence.
    /// Discontinuities use at most a one-millisecond transition on the persisted grid.
    /// Bypassed curves are remapped too, so re-enabling them remains meaningful.
    /// # Errors
    /// Rejects malformed input or a remapping exceeding the keyframe limit; callers
    /// must keep the prior source revision rather than truncate authored evidence.
    pub fn remap_source_edits(
        &self,
        old: &EditTimeline,
        new: &EditTimeline,
    ) -> Result<Self, SoundAssemblyError> {
        self.validate()?;
        if self.points.is_empty() || old == new {
            return Ok(self.clone());
        }
        let old_spans = spans(old);
        let new_spans = spans(new);
        if old_spans == new_spans && old.output_duration_millis() == new.output_duration_millis() {
            return Ok(self.clone());
        }
        let mut points = BTreeMap::new();
        for new_span in new_spans {
            let mut cursor = new_span.original_start;
            for old_span in &old_spans {
                let start = new_span.original_start.max(old_span.original_start);
                let end = new_span.original_end.min(old_span.original_end);
                if end <= start {
                    continue;
                }
                if start > cursor {
                    points.insert(new_span.output_start + cursor - new_span.original_start, 0);
                    points.insert(
                        new_span.output_start + start - new_span.original_start - 1,
                        0,
                    );
                }
                let old_start = old_span.output_start + start - old_span.original_start;
                let new_start = new_span.output_start + start - new_span.original_start;
                points.insert(new_start, self.gain_at(old_start));
                for point in &self.points {
                    if point.source_millis > old_start
                        && point.source_millis < old_start + end - start
                    {
                        points.insert(
                            new_start + point.source_millis - old_start,
                            point.gain_centibels,
                        );
                    }
                }
                points.insert(
                    new_start + end - start - 1,
                    self.gain_at(old_start + end - start - 1),
                );
                points.insert(
                    new_start + end - start,
                    self.gain_at(old_start + end - start),
                );
                cursor = end;
            }
            if cursor < new_span.original_end {
                points.insert(new_span.output_start + cursor - new_span.original_start, 0);
                points.insert(
                    new_span.output_start + new_span.original_end - new_span.original_start,
                    0,
                );
            }
        }
        let result = Self {
            enabled: self.enabled,
            points: points
                .into_iter()
                .map(|(source_millis, gain_centibels)| GainEnvelopePoint {
                    source_millis,
                    gain_centibels,
                })
                .collect(),
        };
        result.validate()?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests;
