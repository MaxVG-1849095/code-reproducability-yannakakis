select count(*) from dblp11, dblp7, dblp8, dblp22, dblp23, dblp20 where dblp11.s = dblp7.s and dblp7.s = dblp8.s and dblp8.s = dblp22.s and dblp22.s = dblp23.s and dblp23.s = dblp20.s;