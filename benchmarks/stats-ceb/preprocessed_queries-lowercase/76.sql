select count(*) from c, p, ph, v, b, u where u.id = p.owneruserid and u.id = b.userid and p.id = c.postid and p.id = ph.postid and p.id = v.postid and p.posttypeid=1 and p.viewcount<=4159 and p.commentcount>=0 and p.commentcount<=12 and ph.posthistorytypeid=2 and v.votetypeid=2 and u.views<=86 and u.downvotes>=0 and u.downvotes<=1 and u.upvotes<=6;
