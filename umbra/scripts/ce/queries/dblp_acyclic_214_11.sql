select count(*) from dblp22, dblp7, dblp21, dblp19, dblp6, dblp5, dblp8, dblp18 where dblp22.s = dblp7.s and dblp7.s = dblp21.s and dblp21.s = dblp19.s and dblp19.s = dblp6.s and dblp6.s = dblp5.s and dblp5.d = dblp8.s and dblp8.d = dblp18.s;