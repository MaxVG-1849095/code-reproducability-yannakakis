select count(*) from c, p, pl, ph, v, u where u.id = p.owneruserid and p.id = v.postid and p.id = c.postid and p.id = pl.postid and p.id = ph.postid and c.creationdate>='2010-07-26 19:37:03'::timestamp and p.score>=-2 and p.commentcount<=18 and p.creationdate>='2010-07-21 13:50:08'::timestamp and p.creationdate<='2014-09-11 00:53:10'::timestamp and pl.creationdate<='2014-08-05 18:27:51'::timestamp and ph.creationdate>='2010-11-27 03:38:45'::timestamp and u.downvotes>=0 and u.upvotes>=0;
