select count(*) from dblp1, dblp6, dblp7, dblp9, dblp17, dblp5, dblp8, dblp18 where dblp1.s = dblp6.s and dblp6.s = dblp7.s and dblp7.s = dblp9.s and dblp9.s = dblp17.s and dblp17.s = dblp5.s and dblp5.d = dblp8.s and dblp8.d = dblp18.s;