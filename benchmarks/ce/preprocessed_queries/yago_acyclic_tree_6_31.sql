select count(*) from yago46, yago17_1, yago5, yago21, yago13, yago17_5 where yago46.s = yago5.d and yago5.d = yago17_5.d and yago46.d = yago17_1.d and yago17_1.d = yago13.d and yago5.s = yago21.s;