select count(*) from c, p, pl, ph, v, u where p.id = pl.postid and p.id = ph.postid and p.id = c.postid and u.id = c.userid and u.id = v.userid and c.score=0 and c.creationdate>='2010-07-20 06:26:28'::timestamp and c.creationdate<='2014-09-11 18:45:09'::timestamp and p.posttypeid=1 and p.favoritecount>=0 and p.favoritecount<=2 and ph.posthistorytypeid=5 and u.downvotes<=0 and u.upvotes>=0 and u.creationdate>='2010-09-18 01:58:41'::timestamp;
