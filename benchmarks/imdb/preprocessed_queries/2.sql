SELECT *
FROM mi,
     mk,
     t
WHERE t.id = mi.movie_id
  AND t.id = mk.movie_id
  AND mk.movie_id = mi.movie_id;

