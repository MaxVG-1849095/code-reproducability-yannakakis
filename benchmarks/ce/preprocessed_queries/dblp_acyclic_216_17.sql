select count(*) from dblp1, dblp25, dblp16, dblp5, dblp21, dblp20, dblp3, dblp7 where dblp1.d = dblp25.d and dblp25.s = dblp16.s and dblp16.d = dblp5.s and dblp5.d = dblp21.d and dblp21.s = dblp20.s and dblp20.s = dblp3.s and dblp3.s = dblp7.s;