select count(*) from dblp18, dblp24, dblp23, dblp4, dblp22, dblp5, dblp8, dblp1 where dblp18.s = dblp24.s and dblp24.s = dblp23.s and dblp23.s = dblp4.s and dblp4.s = dblp22.s and dblp22.s = dblp5.s and dblp5.d = dblp8.s and dblp8.d = dblp1.s;