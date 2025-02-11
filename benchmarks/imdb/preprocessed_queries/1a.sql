SELECT t.id,
         t.title
FROM mi_idx,
     t
WHERE mi_idx.movie_id = t.id ;