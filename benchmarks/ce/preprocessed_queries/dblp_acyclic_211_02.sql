select count(*) from dblp18, dblp22, dblp24, dblp1, dblp12, dblp3, dblp5 where dblp18.s = dblp22.s and dblp22.d = dblp24.d and dblp24.s = dblp1.s and dblp1.d = dblp12.d and dblp12.s = dblp3.s and dblp3.d = dblp5.s;