select count(*) from dblp2, dblp23, dblp20, dblp8, dblp9, dblp19, dblp7, dblp6 where dblp2.s = dblp23.s and dblp23.s = dblp20.s and dblp20.s = dblp8.s and dblp8.d = dblp9.s and dblp9.d = dblp19.s and dblp19.d = dblp7.s and dblp7.s = dblp6.s;