select count(*) from dblp24, dblp5, dblp21, dblp19, dblp8, dblp25 where dblp24.s = dblp5.s and dblp5.d = dblp21.d and dblp21.s = dblp19.s and dblp19.d = dblp8.s and dblp8.d = dblp25.s;