select count(*) from dblp23, dblp2, dblp7, dblp18, dblp5, dblp25, dblp22 where dblp23.s = dblp2.s and dblp2.s = dblp7.s and dblp7.s = dblp18.s and dblp18.d = dblp5.s and dblp5.d = dblp25.s and dblp25.s = dblp22.s;