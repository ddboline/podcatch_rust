CREATE TABLE google_music_metadata (
    id TEXT NOT NULL,
    title TEXT NOT NULL,
    album TEXT NOT NULL,
    artist TEXT NOT NULL,
    track_size INTEGER NOT NULL,
    album_artist TEXT,
    track_number INTEGER,
    disc_number INTEGER,
    total_disc_count INTEGER,
    filename TEXT,
    CONSTRAINT google_music_metadata_id UNIQUE (id)
)
