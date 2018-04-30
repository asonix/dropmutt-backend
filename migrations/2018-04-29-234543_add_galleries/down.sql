-- This file should undo anything in `up.sql`
ALTER TABLE unprocessed_images DROP COLUMN gallery_id;
DROP TABLE gallery_images;
DROP TABLE galleries;
