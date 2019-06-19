CREATE TABLE episodes (
    castid SERIAL PRIMARY KEY,
    episodeid INTEGER NOT NULL,
    title TEXT NOT NULL,
    epurl TEXT NOT NULL,
    enctype TEXT NOT NULL,
    status TEXT NOT NULL,
    epguid TEXT
)
