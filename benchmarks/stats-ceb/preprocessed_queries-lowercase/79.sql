select count(*) from c, p, ph, v, b, u where u.id = p.owneruserid and u.id = b.userid and p.id = c.postid and p.id = ph.postid and p.id = v.postid and c.score=0 and p.score<=21 and p.answercount<=3 and p.favoritecount>=0 and v.creationdate>='2010-07-19 00:00:00'::timestamp and b.date<='2014-09-11 18:35:08'::timestamp and u.reputation<=240;
