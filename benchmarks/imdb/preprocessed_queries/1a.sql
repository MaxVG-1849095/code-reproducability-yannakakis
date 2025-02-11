SELECT t.id,
         t.title
FROM mi_idx,
     t
WHERE t.id = mi_idx.movie_id;