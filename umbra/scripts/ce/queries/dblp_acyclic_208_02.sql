select count(*) from dblp18, dblp1, dblp21, dblp9, dblp3, dblp17, dblp2 where dblp18.s = dblp1.s and dblp1.s = dblp21.s and dblp21.s = dblp9.s and dblp9.d = dblp3.s and dblp3.d = dblp17.s and dblp17.s = dblp2.s;