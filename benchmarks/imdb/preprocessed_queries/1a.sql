SELECT count(t.id)
FROM mi_idx,
     t
WHERE mi_idx.movie_id = t.id ;