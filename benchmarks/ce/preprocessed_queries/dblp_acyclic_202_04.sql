select count(*) from dblp2, dblp6, dblp1, dblp8, dblp9, dblp21 where dblp2.s = dblp6.s and dblp6.s = dblp1.s and dblp1.s = dblp8.s and dblp8.s = dblp9.s and dblp9.d = dblp21.s;