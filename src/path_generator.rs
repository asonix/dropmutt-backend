use std::{path::PathBuf, sync::{Arc, atomic::{AtomicUsize, Ordering}}};

use rand::{thread_rng, Rng};

#[derive(Clone)]
pub struct PathGenerator {
    next_path: Arc<AtomicUsize>,
}

impl PathGenerator {
    pub fn with_start_position(start: usize) -> Self {
        PathGenerator {
            next_path: Arc::new(AtomicUsize::new(start)),
        }
    }

    pub fn next_path(&self, extension: &str) -> PathBuf {
        let path_id = self.next_path.fetch_add(1, Ordering::Relaxed);

        let (_, mut sections) = (0..3).fold((path_id, Vec::new()), |(path_id, mut sections), _| {
            sections.push(format!("{:03}", path_id % 1000));
            (path_id / 1000, sections)
        });

        sections.reverse();

        let filename: String = thread_rng().gen_ascii_chars().take(10).collect();

        let mut file_path = sections
            .into_iter()
            .fold(PathBuf::new(), |mut path, section| {
                path.push(section);
                path
            });

        file_path.push(format!("{}.{}", filename, extension));

        file_path
    }
}

#[cfg(test)]
mod tests {
    use super::PathGenerator;

    #[test]
    fn generates_correct_paths() {
        let image_path = PathGenerator::with_start_position(0);

        assert!(
            image_path
                .next_path("png")
                .to_str()
                .unwrap()
                .starts_with("000/000/000")
        );

        for _ in 0..999 {
            image_path.next_path("png");
        }

        assert!(
            image_path
                .next_path("png")
                .to_str()
                .unwrap()
                .starts_with("000/001/000")
        );

        for _ in 0..499 {
            image_path.next_path("png");
        }

        assert!(
            image_path
                .next_path("png")
                .to_str()
                .unwrap()
                .starts_with("000/001/500")
        );

        for _ in 0..499 {
            image_path.next_path("png");
        }

        assert!(
            image_path
                .next_path("png")
                .to_str()
                .unwrap()
                .starts_with("000/002/000")
        );

        for _ in 0..(1_000_000 - 2_001) {
            image_path.next_path("png");
        }

        assert!(
            image_path
                .next_path("png")
                .to_str()
                .unwrap()
                .starts_with("001/000/000")
        );
    }
}
