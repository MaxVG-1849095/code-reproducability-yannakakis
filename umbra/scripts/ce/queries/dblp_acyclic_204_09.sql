select count(*) from dblp25, dblp20, dblp8, dblp1, dblp26, dblp7 where dblp25.s = dblp20.s and dblp20.s = dblp8.s and dblp8.d = dblp1.s and dblp1.d = dblp26.s and dblp26.d = dblp7.s;