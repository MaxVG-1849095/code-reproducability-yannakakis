SELECT MIN(a1.name) AS writer_pseudo_name,
       MIN(t.title) AS movie_title
FROM a1,
     ci,
     cn,
     mc,
     n1,
     rt,
     t
WHERE cn.country_code ='[us]'
  AND rt.role ='writer'
  AND a1.person_id = n1.id
  AND n1.id = ci.person_id
  AND ci.movie_id = t.id
  AND t.id = mc.movie_id
  AND mc.company_id = cn.id
  AND ci.role_id = rt.id
  AND a1.person_id = ci.person_id
  AND ci.movie_id = mc.movie_id;

