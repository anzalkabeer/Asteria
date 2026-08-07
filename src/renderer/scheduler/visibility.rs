use crate::segment::ViewportSegment;

pub fn cull_invisible_segments(
    segments: &[ViewportSegment],
    viewport: [f32; 4],
) -> Vec<&ViewportSegment> {
    segments
        .iter()
        .filter(|seg| {
            let (sx, sy, sw, sh) = (seg.rect.x, seg.rect.y, seg.rect.width, seg.rect.height);
            let (vx, vy, vw, vh) = (viewport[0], viewport[1], viewport[2], viewport[3]);

            // Intersection test
            sx < vx + vw && sx + sw > vx && sy < vy + vh && sy + sh > vy
        })
        .collect()
}
