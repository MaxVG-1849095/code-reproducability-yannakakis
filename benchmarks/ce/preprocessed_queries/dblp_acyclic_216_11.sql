select count(*) from dblp5, dblp21, dblp16, dblp1, dblp12, dblp7, dblp20, dblp19 where dblp5.d = dblp21.d and dblp21.s = dblp16.s and dblp16.d = dblp1.s and dblp1.d = dblp12.d and dblp12.s = dblp7.s and dblp7.s = dblp20.s and dblp20.s = dblp19.s;