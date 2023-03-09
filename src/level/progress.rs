use super::*;

impl Level {
    pub fn progress_at(&self, pos: vec2<f32>) -> Option<f32> {
        let mut total_len = 0.0;
        for path in &self.expected_path {
            for window in path.windows(2) {
                let a = window[0];
                let b = window[1];
                total_len += (b - a).len();
            }
        }
        let mut progress = None;
        let mut closest_point_distance = self.max_progress_distance;
        let mut prefix_len = 0.0;

        for path in &self.expected_path {
            for window in path.windows(2) {
                let a = window[0];
                let b = window[1];
                let v = Surface {
                    p1: a,
                    p2: b,
                    flow: 0.0,
                    type_name: String::new(),
                }
                .vector_from(pos);
                if v.len() < closest_point_distance {
                    closest_point_distance = v.len();
                    progress = Some((prefix_len + (pos + v - a).len()) / total_len);
                }
                prefix_len += (b - a).len();
            }
        }
        progress
    }
}
