select count(*) from dblp7, dblp17, dblp22, dblp1, dblp23, dblp9, dblp2 where dblp7.s = dblp17.s and dblp17.s = dblp22.s and dblp22.s = dblp1.s and dblp1.s = dblp23.s and dblp23.s = dblp9.s and dblp9.d = dblp2.s;