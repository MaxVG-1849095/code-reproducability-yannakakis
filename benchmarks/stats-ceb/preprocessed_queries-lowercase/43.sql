select count(*) from ph, p, u, b where u.id = p.owneruserid and p.owneruserid = ph.userid and ph.userid = b.userid and ph.posthistorytypeid=3 and p.score>=-7 and u.reputation>=1 and u.upvotes>=0 and u.upvotes<=117;
