select count(*) from c, p, pl, ph, v where p.id = pl.postid and p.id = v.postid and p.id = ph.postid and p.id = c.postid and c.creationdate>='2010-08-01 12:12:41'::timestamp and p.score<=44 and p.favoritecount>=0 and p.favoritecount<=3 and p.creationdate>='2010-08-11 13:53:56'::timestamp and p.creationdate<='2014-09-03 11:52:36'::timestamp and pl.linktypeid=1 and pl.creationdate<='2014-08-11 17:26:31'::timestamp and ph.creationdate>='2010-09-20 19:11:45'::timestamp and v.creationdate<='2014-09-11 00:00:00'::timestamp;
