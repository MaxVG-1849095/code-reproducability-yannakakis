select count(*) from dblp18, dblp8, dblp25, dblp21, dblp7, dblp1, dblp17 where dblp18.s = dblp8.s and dblp8.s = dblp25.s and dblp25.s = dblp21.s and dblp21.s = dblp7.s and dblp7.s = dblp1.s and dblp1.s = dblp17.s;