select count(*) from dblp5, dblp21, dblp17, dblp1, dblp12, dblp13, dblp6, dblp23 where dblp5.d = dblp21.d and dblp21.s = dblp17.s and dblp17.d = dblp1.s and dblp1.d = dblp12.d and dblp12.s = dblp13.s and dblp13.s = dblp6.s and dblp6.s = dblp23.s;