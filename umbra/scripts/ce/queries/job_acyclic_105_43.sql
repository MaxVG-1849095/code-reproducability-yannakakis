select count(*) from imdb100, imdb125, imdb23, imdb50 where imdb100.d = imdb125.d and imdb125.d = imdb23.s and imdb23.s = imdb50.s;