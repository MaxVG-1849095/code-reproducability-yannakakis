select count(*) from c, pl, ph, v, p where pl.postid = p.id and c.postid = p.id and v.postid = p.id and ph.postid = p.id and pl.linktypeid=1 and pl.creationdate>='2011-06-14 13:31:35'::timestamp and v.creationdate>='2010-07-19 00:00:00'::timestamp and v.creationdate<='2014-09-10 00:00:00'::timestamp;
