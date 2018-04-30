-- Your SQL goes here
CREATE TABLE galleries (
    id SERIAL PRIMARY KEY,
    name VARCHAR(120) UNIQUE NOT NULL,
    nsfw BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT 'now',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT 'now'
);

INSERT INTO galleries (name) VALUES ( 'sketches' );
INSERT INTO galleries (name) VALUES ( 'flat-colors' );
INSERT INTO galleries (name) VALUES ( 'cell-shaded' );

CREATE TABLE gallery_images (
    id SERIAL PRIMARY KEY,
    gallery_id INTEGER REFERENCES galleries(id) ON DELETE CASCADE NOT NULL,
    image_id INTEGER REFERENCES images(id) ON DELETE CASCADE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT 'now',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT 'now'
);
