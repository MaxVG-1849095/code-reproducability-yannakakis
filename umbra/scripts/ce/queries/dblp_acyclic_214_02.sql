select count(*) from dblp6, dblp5, dblp23, dblp25, dblp1, dblp21, dblp9, dblp7 where dblp6.s = dblp5.s and dblp5.s = dblp23.s and dblp23.s = dblp25.s and dblp25.s = dblp1.s and dblp1.s = dblp21.s and dblp21.d = dblp9.s and dblp9.d = dblp7.s;