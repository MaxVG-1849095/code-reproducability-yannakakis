select count(*) from yago2_0, yago2_1, yago21, yago54, yago58, yago4 where yago2_0.s = yago2_1.s and yago2_1.d = yago21.d and yago21.s = yago54.d and yago54.s = yago58.s and yago58.d = yago4.s;