select count(*) from c, ph, b, u where u.id = c.userid and u.id = ph.userid and u.id = b.userid and c.score=0 and c.creationdate>='2010-07-20 10:52:57'::timestamp and ph.posthistorytypeid=5 and ph.creationdate>='2011-01-31 15:35:37'::timestamp and u.reputation>=1 and u.reputation<=356 and u.downvotes<=34 and u.creationdate>='2010-07-19 21:29:29'::timestamp and u.creationdate<='2014-08-20 14:31:46'::timestamp;
