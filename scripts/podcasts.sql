CREATE TABLE podcasts (
    castid SERIAL PRIMARY KEY,
    castname TEXT NOT NULL,
    feedurl TEXT NOT NULL,
    directory TEXT
)
