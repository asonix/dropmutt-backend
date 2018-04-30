-- Your SQL goes here
CREATE TABLE files (
    id SERIAL PRIMARY KEY,
    file_path VARCHAR(512) UNIQUE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT 'now',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT 'now'
);

CREATE TABLE images (
    id SERIAL PRIMARY KEY,
    uploaded_by INTEGER REFERENCES users(id) ON DELETE CASCADE NOT NULL,
    description TEXT NOT NULL,
    alternate_text TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT 'now',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT 'now'
);

CREATE TABLE image_files (
    id SERIAL PRIMARY KEY,
    image_id INTEGER REFERENCES images(id) ON DELETE CASCADE NOT NULL,
    file_id INTEGER REFERENCES files(id) ON DELETE CASCADE NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT 'now',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT 'now'
);

CREATE TABLE unprocessed_images (
    id SERIAL PRIMARY KEY,
    uploaded_by INTEGER REFERENCES users(id) ON DELETE CASCADE NOT NULL,
    image_file INTEGER REFERENCES files(id) ON DELETE CASCADE NOT NULL,
    description TEXT NOT NULL,
    alternate_text TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT 'now',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT 'now'
);
