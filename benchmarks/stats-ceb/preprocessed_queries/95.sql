SELECT COUNT(*) FROM c, ph, v, p WHERE ph.PostId = p.Id AND c.PostId = p.Id AND v.PostId = p.Id AND v.CreationDate<='2014-09-12 00:00:00'::timestamp;
