select count(*) from c, p, ph, v, u where v.userid = u.id and c.userid = u.id and p.owneruserid = u.id and ph.userid = u.id and c.score=2 and p.answercount>=0 and p.answercount<=9 and p.creationdate>='2010-07-20 18:17:25'::timestamp and p.creationdate<='2014-08-26 12:57:22'::timestamp and ph.creationdate<='2014-09-02 07:58:47'::timestamp and v.bountyamount>=0 and v.creationdate>='2010-05-19 00:00:00'::timestamp and u.upvotes<=230 and u.creationdate>='2010-09-22 01:07:10'::timestamp and u.creationdate<='2014-08-15 05:52:23'::timestamp;
