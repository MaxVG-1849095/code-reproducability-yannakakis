select count(*) from dblp17, dblp25, dblp22, dblp9, dblp3, dblp18, dblp5, dblp21 where dblp17.s = dblp25.s and dblp25.s = dblp22.s and dblp22.s = dblp9.s and dblp9.d = dblp3.s and dblp3.d = dblp18.s and dblp18.d = dblp5.s and dblp5.s = dblp21.s;