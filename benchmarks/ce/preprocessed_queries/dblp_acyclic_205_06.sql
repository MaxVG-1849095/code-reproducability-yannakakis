select count(*) from dblp12, dblp5, dblp21, dblp3, dblp8, dblp22 where dblp12.s = dblp5.s and dblp5.d = dblp21.d and dblp21.s = dblp3.s and dblp3.d = dblp8.s and dblp8.d = dblp22.s;