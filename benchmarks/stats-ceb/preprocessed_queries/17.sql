SELECT COUNT(*) FROM p, t, v WHERE p.Id = t.ExcerptPostId AND p.OwnerUserId = v.UserId AND p.CreationDate>='2010-07-20 02:01:05'::timestamp;
