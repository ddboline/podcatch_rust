CREATE TABLE episodes (
    castid INTEGER NOT NULL,
    episodeid INTEGER NOT NULL,
    title TEXT NOT NULL,
    epurl TEXT NOT NULL,
    enctype TEXT NOT NULL,
    status TEXT NOT NULL,
    epguid TEXT,
    CONSTRAINT episodes_castid_episodeid_key UNIQUE (castid, episodeid),
    CONSTRAINT episodes_castid_epurl_key UNIQUE (castid, epurl)
)
