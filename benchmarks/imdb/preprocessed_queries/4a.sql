SELECT MIN(mi_idx.info) AS rating,
       MIN(t.title) AS movie_title
FROM it,
     k,
     mi_idx,
     mk,
     t
WHERE it.info ='rating'
  AND k.keyword LIKE '%sequel%'
  AND mi_idx.info > '5.0'
  AND t.production_year > 2005
  AND t.id = mi_idx.movie_id
  AND t.id = mk.movie_id
  AND mk.movie_id = mi_idx.movie_id
  AND k.id = mk.keyword_id
  AND it.id = mi_idx.info_type_id;

