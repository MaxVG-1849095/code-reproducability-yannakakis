select count(*) from ph, p, u, b where b.userid = u.id and p.owneruserid = u.id and ph.userid = u.id and ph.posthistorytypeid=5 and p.viewcount>=0 and p.viewcount<=2024 and u.reputation>=1 and u.reputation<=750 and b.date>='2010-07-20 10:34:10'::timestamp;
