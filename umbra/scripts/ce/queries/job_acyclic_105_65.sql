select count(*) from imdb100, imdb125, imdb82, imdb52 where imdb100.d = imdb125.d and imdb125.d = imdb82.s and imdb82.s = imdb52.s;