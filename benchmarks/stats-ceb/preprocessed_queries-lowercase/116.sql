select count(*) from c, ph, b, u where u.id = b.userid and u.id = ph.userid and u.id = c.userid and c.score=2 and ph.creationdate>='2010-08-19 12:45:55'::timestamp and ph.creationdate<='2014-09-03 21:46:37'::timestamp and u.reputation>=1 and u.reputation<=1183 and u.views>=0;
